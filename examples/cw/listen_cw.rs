use futuresdr::anyhow::Context;
use futuresdr::anyhow::Result;
use futuresdr::blocks::audio::AudioSink;
use futuresdr::blocks::audio::Oscillator;
use futuresdr::blocks::Combine;
use futuresdr::blocks::ApplyIntoIter;
use futuresdr::blocks::VectorSourceBuilder;
use futuresdr::blocks::DisplaySink;
use futuresdr::runtime::Flowgraph;
use futuresdr::runtime::Runtime;
use std::time;
use std::fmt;

#[derive(Debug, Copy, Clone)]
pub enum CWAlphabet {
    Dot,
    Dash,
    LetterSpace,
    WordSpace,
}

impl fmt::Display for CWAlphabet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CWAlphabet::Dot => write!(f, "."),
            CWAlphabet::Dash => write!(f, "-"),
            CWAlphabet::LetterSpace => write!(f, " "),
            CWAlphabet::WordSpace => write!(f, " / "),
        }
    }
}

fn morse(i: &char) -> Vec<CWAlphabet> {
    let c = *i;
    if c == 'A' {
        return vec![CWAlphabet::Dot, CWAlphabet::Dash, CWAlphabet::LetterSpace];
    } else if c == 'B'  {
        return vec![CWAlphabet::Dash, CWAlphabet::Dot, CWAlphabet::Dot, CWAlphabet::Dot, CWAlphabet::LetterSpace];
    } else if c == 'C'  {
        return vec![CWAlphabet::Dash, CWAlphabet::Dot, CWAlphabet::Dash, CWAlphabet::Dot, CWAlphabet::LetterSpace];
    } else if c == 'D'  {
        return vec![CWAlphabet::Dash, CWAlphabet::Dot, CWAlphabet::Dot, CWAlphabet::LetterSpace];
    } else if c == 'E'  {
        return vec![CWAlphabet::Dot, CWAlphabet::LetterSpace];
    } else if c == 'F'  {
        return vec![CWAlphabet::Dot, CWAlphabet::Dot, CWAlphabet::Dash, CWAlphabet::Dot, CWAlphabet::LetterSpace];
    } else if c == 'G'  {
        return vec![CWAlphabet::Dash, CWAlphabet::Dash, CWAlphabet::Dot, CWAlphabet::LetterSpace];
    } else if c == 'H'  {
        return vec![CWAlphabet::Dot, CWAlphabet::Dot, CWAlphabet::Dot, CWAlphabet::Dot, CWAlphabet::LetterSpace];
    } else if c == 'I'  {
        return vec![CWAlphabet::Dot, CWAlphabet::Dot, CWAlphabet::LetterSpace];
    } else if c == 'J'  {
        return vec![CWAlphabet::Dot, CWAlphabet::Dash, CWAlphabet::Dash, CWAlphabet::Dash, CWAlphabet::LetterSpace];
    } else if c == 'K'  {
        return vec![CWAlphabet::Dash, CWAlphabet::Dot, CWAlphabet::Dash, CWAlphabet::LetterSpace];
    } else if c == 'L'  {
        return vec![CWAlphabet::Dot, CWAlphabet::Dash, CWAlphabet::Dot, CWAlphabet::Dash, CWAlphabet::LetterSpace];
    } else if c == 'M'  {
        return vec![CWAlphabet::Dot, CWAlphabet::Dash, CWAlphabet::Dot, CWAlphabet::Dot, CWAlphabet::LetterSpace];
    } else if c == 'N'  {
        return vec![CWAlphabet::Dash, CWAlphabet::Dot, CWAlphabet::LetterSpace];
    } else if c == 'O'  {
        return vec![CWAlphabet::Dash, CWAlphabet::Dash, CWAlphabet::Dash, CWAlphabet::LetterSpace];
    } else if c == 'P'  {
        return vec![CWAlphabet::Dot, CWAlphabet::Dash, CWAlphabet::Dash, CWAlphabet::Dot, CWAlphabet::LetterSpace];
    } else if c == 'Q'  {
        return vec![CWAlphabet::Dash, CWAlphabet::Dash, CWAlphabet::Dot, CWAlphabet::Dash, CWAlphabet::LetterSpace];
    } else if c == 'R'  {
        return vec![CWAlphabet::Dot, CWAlphabet::Dash, CWAlphabet::Dot, CWAlphabet::LetterSpace];
    } else if c == 'S'  {
        return vec![CWAlphabet::Dot, CWAlphabet::Dot, CWAlphabet::Dot, CWAlphabet::LetterSpace];
    } else if c == 'T'  {
        return vec![CWAlphabet::Dash, CWAlphabet::LetterSpace];
    } else if c == 'U'  {
        return vec![CWAlphabet::Dot, CWAlphabet::Dot, CWAlphabet::Dash, CWAlphabet::LetterSpace];
    } else if c == 'V'  {
        return vec![CWAlphabet::Dot, CWAlphabet::Dot, CWAlphabet::Dot, CWAlphabet::Dash, CWAlphabet::LetterSpace];
    } else if c == 'W'  {
        return vec![CWAlphabet::Dot, CWAlphabet::Dash, CWAlphabet::Dash, CWAlphabet::LetterSpace];
    } else if c == 'X'  {
        return vec![CWAlphabet::Dash, CWAlphabet::Dot, CWAlphabet::Dot, CWAlphabet::Dash, CWAlphabet::LetterSpace];
    } else if c == 'Y'  {
        return vec![CWAlphabet::Dash, CWAlphabet::Dot, CWAlphabet::Dash, CWAlphabet::Dash, CWAlphabet::LetterSpace];
    } else if c == 'Z'  {
        return vec![CWAlphabet::Dash, CWAlphabet::Dash, CWAlphabet::Dot, CWAlphabet::Dot, CWAlphabet::LetterSpace];
    } else /*if c == ' '*/ {
        return vec![CWAlphabet::WordSpace];
    }
}

const SAMPLE_RATE: usize = 48_000;
const SIDETONE_FREQ: u32 = 440; // Usually between 400Hz and 750Hz
const DOT_LENGTH: usize = SAMPLE_RATE / 10;

impl IntoIterator for CWAlphabet {
    type Item = f32;
    type IntoIter = std::iter::Chain<std::iter::Take<std::iter::Repeat<f32>>, std::iter::Take<std::iter::Repeat<f32>>>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            CWAlphabet::Dot => std::iter::repeat(1.0).take(DOT_LENGTH).chain(std::iter::repeat(0.0).take(DOT_LENGTH)),
            CWAlphabet::Dash => std::iter::repeat(1.0).take(3*DOT_LENGTH).chain(std::iter::repeat(0.0).take(DOT_LENGTH)),
            CWAlphabet::LetterSpace => std::iter::repeat(0.0).take(3*DOT_LENGTH).chain(std::iter::repeat(0.0).take(0)),
            CWAlphabet::WordSpace => std::iter::repeat(0.0).take((5-2)*DOT_LENGTH).chain(std::iter::repeat(0.0).take(0)),
        }
    }
}

fn main() -> Result<()> {
    let mut fg = Flowgraph::new();


    let orig: Vec<char> = vec!['C', 'Q', ' ', 'C', 'Q', ' ', 'C', 'Q', ' ', 'L', 'O', 'I', 'C'];

    let src = fg.add_block(VectorSourceBuilder::<char>::new(orig).build());
    let audio_snk = fg.add_block(AudioSink::new(SAMPLE_RATE.try_into().unwrap(), 1));
    let morse = fg.add_block(ApplyIntoIter::<char, Vec<CWAlphabet>>::new(&morse));
    let switch_command = fg.add_block(ApplyIntoIter::<CWAlphabet, CWAlphabet>::new(
        |c: &CWAlphabet| {
            return *c;
        }
    ));
    let sidetone_src = fg.add_block( Oscillator::new(SIDETONE_FREQ, 0.2));
    let switch_sidetone = fg.add_block(Combine::new(|a: &f32, b: &f32| -> f32 { *a * *b }));
    let snk = fg.add_block(DisplaySink::<CWAlphabet>::new());

    fg.connect_stream(src, "out", morse, "in")?;
    fg.connect_stream(morse, "out", snk, "in")?;
    fg.connect_stream(morse, "out", switch_command, "in")?;
    fg.connect_stream(switch_command, "out", switch_sidetone, "in0")?;
    fg.connect_stream(sidetone_src, "out", switch_sidetone, "in1")?;
    fg.connect_stream(switch_sidetone, "out", audio_snk, "in")?;

    // let debug_snk = fg.add_block(DisplaySink::<f32>::new());
    // fg.connect_stream(switch_command, "out", debug_snk, "in")?;

    let now = time::Instant::now();
    fg = Runtime::new().run(fg)?;
    let elapsed = now.elapsed();

    println!("\nflowgraph took {:?}", elapsed);

    Ok(())
}
