use futures::channel::mpsc::Sender;
use futures::channel::oneshot;
use futures::SinkExt;
use std::cmp::PartialEq;
use std::fmt::Debug;
use std::hash::Hash;

#[cfg(not(target_arch = "wasm32"))]
use crate::runtime::buffer::circular::Circular;
#[cfg(target_arch = "wasm32")]
use crate::runtime::buffer::slab::Slab;
use crate::runtime::buffer::BufferBuilder;
use crate::runtime::buffer::BufferWriter;
use crate::runtime::Block;
use crate::runtime::BlockDescription;
use crate::runtime::BlockMessage;
use crate::runtime::Error;
use crate::runtime::FlowgraphDescription;
use crate::runtime::FlowgraphMessage;
use crate::runtime::Kernel;
use crate::runtime::Pmt;
use crate::runtime::PortId;
use crate::runtime::Topology;

/// The main component of any FutureSDR program.
///
/// A [Flowgraph] is what drives the entire program. It is composed of a set of blocks and connections between them.
/// There is at least one source and one sink in every Flowgraph.
pub struct Flowgraph {
    pub(crate) topology: Option<Topology>,
}

impl Flowgraph {
    /// Creates a new [Flowgraph] with an empty [Topology]
    pub fn new() -> Flowgraph {
        Flowgraph {
            topology: Some(Topology::new()),
        }
    }

    /// Add [`Block`] to flowgraph
    pub fn add_block(&mut self, block: impl Into<Block>) -> Result<usize, Error> {
        self.topology.as_mut().unwrap().add_block(block.into())
    }

    /// Make stream connection
    pub fn connect_stream(
        &mut self,
        src_block: usize,
        src_port: impl Into<PortId>,
        dst_block: usize,
        dst_port: impl Into<PortId>,
    ) -> Result<(), Error> {
        self.topology.as_mut().unwrap().connect_stream(
            src_block,
            src_port.into(),
            dst_block,
            dst_port.into(),
            DefaultBuffer::new(),
        )
    }

    /// Make stream connection, using the given buffer
    pub fn connect_stream_with_type<B: BufferBuilder + Debug + Eq + Hash>(
        &mut self,
        src_block: usize,
        src_port: impl Into<PortId>,
        dst_block: usize,
        dst_port: impl Into<PortId>,
        buffer: B,
    ) -> Result<(), Error> {
        self.topology.as_mut().unwrap().connect_stream(
            src_block,
            src_port.into(),
            dst_block,
            dst_port.into(),
            buffer,
        )
    }

    /// Make message connection
    pub fn connect_message(
        &mut self,
        src_block: usize,
        src_port: impl Into<PortId>,
        dst_block: usize,
        dst_port: impl Into<PortId>,
    ) -> Result<(), Error> {
        self.topology.as_mut().unwrap().connect_message(
            src_block,
            src_port.into(),
            dst_block,
            dst_port.into(),
        )
    }

    /// Try to get kernel from given block
    pub fn kernel<T: Kernel + 'static>(&self, id: usize) -> Option<&T> {
        self.topology
            .as_ref()
            .and_then(|t| t.block_ref(id))
            .and_then(|b| b.kernel())
    }

    /// Try to get kernel mutably from given block
    pub fn kernel_mut<T: Kernel + 'static>(&mut self, id: usize) -> Option<&T> {
        self.topology
            .as_mut()
            .and_then(|t| t.block_mut(id))
            .and_then(|b| b.kernel_mut())
    }
}

impl Default for Flowgraph {
    fn default() -> Self {
        Self::new()
    }
}

impl AsMut<Flowgraph> for Flowgraph {
    fn as_mut(&mut self) -> &mut Flowgraph {
        self
    }
}

/// Handle to interact with running [`Flowgraph`]
#[derive(Debug, Clone)]
pub struct FlowgraphHandle {
    inbox: Sender<FlowgraphMessage>,
}

impl PartialEq for FlowgraphHandle {
    fn eq(&self, other: &Self) -> bool {
        self.inbox.same_receiver(&other.inbox)
    }
}

impl FlowgraphHandle {
    pub(crate) fn new(inbox: Sender<FlowgraphMessage>) -> FlowgraphHandle {
        FlowgraphHandle { inbox }
    }

    /// Call message handler, ignoring the result
    pub async fn call(
        &mut self,
        block_id: usize,
        port_id: impl Into<PortId>,
        data: Pmt,
    ) -> Result<(), Error> {
        let (tx, rx) = oneshot::channel::<Result<(), Error>>();
        self.inbox
            .send(FlowgraphMessage::BlockCall {
                block_id,
                port_id: port_id.into(),
                data,
                tx,
            })
            .await
            .or(Err(Error::InvalidBlock(block_id)))?;
        rx.await?
    }

    /// Call message handler
    pub async fn callback(
        &mut self,
        block_id: usize,
        port_id: impl Into<PortId>,
        data: Pmt,
    ) -> Result<Pmt, Error> {
        let (tx, rx) = oneshot::channel::<Result<Pmt, Error>>();
        self.inbox
            .send(FlowgraphMessage::BlockCallback {
                block_id,
                port_id: port_id.into(),
                data,
                tx,
            })
            .await
            .map_err(|_| Error::InvalidBlock(block_id))?;
        rx.await?
    }

    /// Get [`FlowgraphDescription`]
    pub async fn description(&mut self) -> Result<FlowgraphDescription, Error> {
        let (tx, rx) = oneshot::channel::<FlowgraphDescription>();
        self.inbox
            .send(FlowgraphMessage::FlowgraphDescription { tx })
            .await
            .or(Err(Error::FlowgraphTerminated))?;
        let d = rx.await.or(Err(Error::FlowgraphTerminated))?;
        Ok(d)
    }

    /// Get [`BlockDescription`]
    pub async fn block_description(&mut self, block_id: usize) -> Result<BlockDescription, Error> {
        let (tx, rx) = oneshot::channel::<Result<BlockDescription, Error>>();
        self.inbox
            .send(FlowgraphMessage::BlockDescription { block_id, tx })
            .await
            .map_err(|_| Error::InvalidBlock(block_id))?;
        let d = rx.await.map_err(|_| Error::InvalidBlock(block_id))??;
        Ok(d)
    }

    /// Send a terminate message to the [`Flowgraph`]
    ///
    /// Does not wait until the [`Flowgraph`] is actually terminated.
    pub async fn terminate(&mut self) -> Result<(), Error> {
        self.inbox
            .send(FlowgraphMessage::Terminate)
            .await
            .map_err(|_| Error::FlowgraphTerminated)?;
        Ok(())
    }

    /// Terminate the [`Flowgraph`]
    ///
    /// Send a terminate message to the [`Flowgraph`] and wait until it is shutdown.
    pub async fn terminate_and_wait(&mut self) -> Result<(), Error> {
        self.terminate()
            .await
            .map_err(|_| Error::FlowgraphTerminated)?;
        while !self.inbox.is_closed() {
            #[cfg(not(target_arch = "wasm32"))]
            async_io::Timer::after(std::time::Duration::from_millis(200)).await;
            #[cfg(target_arch = "wasm32")]
            gloo_timers::future::sleep(std::time::Duration::from_millis(200)).await;
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Hash)]
pub struct DefaultBuffer;

impl Eq for DefaultBuffer {}

impl DefaultBuffer {
    fn new() -> DefaultBuffer {
        DefaultBuffer
    }
}

impl BufferBuilder for DefaultBuffer {
    #[cfg(not(target_arch = "wasm32"))]
    fn build(
        &self,
        item_size: usize,
        writer_inbox: Sender<BlockMessage>,
        writer_output_id: usize,
    ) -> BufferWriter {
        Circular::new().build(item_size, writer_inbox, writer_output_id)
    }
    #[cfg(target_arch = "wasm32")]
    fn build(
        &self,
        item_size: usize,
        writer_inbox: Sender<BlockMessage>,
        writer_output_id: usize,
    ) -> BufferWriter {
        Slab::new().build(item_size, writer_inbox, writer_output_id)
    }
}
