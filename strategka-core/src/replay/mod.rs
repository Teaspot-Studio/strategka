mod decoder;
mod encoder;
pub mod error;

use nom::{
    bytes::streaming::take,
    error::context,
    number::streaming::{be_u32, be_u64},
    Err, Needed,
};
use serde::{de::DeserializeOwned, Serialize};
use std::io::Read;
use std::{fs::File, io::Write, path::Path};

use crate::World;
use error::{Error, GenericError, Result, ResultOwned};

use self::decoder::*;
use self::encoder::*;

/// Each tick simulation has a number from the begining
pub type Turn = u64;

/// Information that can be used to replay the state of simulation
/// back from begining to the last state.
///
/// We assume that simulation is performed at some constant
/// step per second.
#[derive(Debug, PartialEq, Clone)]
pub struct Replay<W: World> {
    /// Simulation turns per second
    pub rate: u32,
    /// Initial state of simulation to start with
    pub initial: W,
    /// All recorded inputs from players or external events
    pub inputs: Vec<(Turn, Vec<W::Input>)>,
}

impl<W: World + Default> Default for Replay<W> {
    fn default() -> Self {
        Replay {
            rate: 60,
            initial: Default::default(),
            inputs: vec![],
        }
    }
}

// Magic bytes to distinguish other files from the replay. Ascii for STGR
const MAGIC_BYTES: [u8; 4] = [0x53, 0x54, 0x47, 0x52];
// Current maximum format version of replays the code supports
const REPLAY_FORMAT_VERSION: u32 = 1;

impl<W: World + Default + Clone + Serialize + DeserializeOwned> Replay<W> {
    /// Create a new replay with given initial state
    pub fn new(world: &W, rate: u32) -> Self {
        Replay {
            initial: world.clone(),
            rate,
            inputs: vec![],
        }
    }

    /// Record inputs from external events
    pub fn record(&mut self, turn: Turn, inputs: &[W::Input]) -> Result<()> {
        if let Some((last_turn, _)) = self.inputs.last() {
            if *last_turn >= turn {
                return Err(Error::IncoherentTurn(*last_turn, turn));
            }
        }
        self.inputs.push((turn, inputs.to_vec()));
        Ok(())
    }

    /// Write down bytes of replay into the file located at given [path]
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let f = File::create(path)?;
        self.encode(f)?;
        Ok(())
    }

    /// Load replay from file
    pub fn load<P: AsRef<Path> + Clone>(path: P) -> ResultOwned<Self> {
        let mut f = File::open(path.clone())?;
        let mut last_needed: Option<Needed> = None;
        let mut buff: Vec<u8> = Vec::new();
        loop {
            const CHUNK_SIZE: usize = 8 * 1014 * 1024; // 8 MB
            let mut chunk: Vec<u8> = vec![0; CHUNK_SIZE];

            let n = f.read(&mut chunk)?;
            if n == 0 {
                if let Some(needed) = last_needed {
                    log::error!(
                        "Cannot parse replay from {:?}, missing {needed:?}",
                        path.as_ref().to_str()
                    );
                    return Err(GenericError::Incomplete(needed));
                }
            }
            buff.extend_from_slice(&chunk[0..n]); // TODO: implement truly incremental parser for large replays
            match Self::parser(&buff) {
                Ok((_, value)) => return Ok(value),
                Err(Err::Incomplete(needed)) => {
                    last_needed = Some(needed);
                }
                Err(Err::Error(e)) => return Err(e.into_owned()),
                Err(Err::Failure(e)) => return Err(e.into_owned()),
            }
        }
    }

    /// Write down serialized bytes of replay into the buffer
    pub fn encode<S: Write>(&self, mut sink: S) -> Result<()> {
        sink.write_all(&MAGIC_BYTES)?;
        encode_be_u32(REPLAY_FORMAT_VERSION, &mut sink)?;
        sink.write_all(&W::magic_bytes())?;
        encode_be_u32(W::current_version(), &mut sink)?;
        encode_be_u32(self.rate, &mut sink)?;
        length_encoded(&mut sink, |sink| ciborium_into_writer(&self.initial, sink))?;
        encode_vec(&self.inputs, &mut sink, |mut sink, (step, inputs)| {
            encode_be_u64(*step, &mut sink)?;
            encode_vec(inputs, &mut sink, |sink, input| {
                length_encoded(sink, |sink| ciborium_into_writer(input, sink))
            })
        })?;
        Ok(())
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        match Self::parser(bytes) {
            Ok((_, value)) => Ok(value),
            Err(Err::Incomplete(needed)) => Err(Error::Incomplete(needed)),
            Err(Err::Error(e)) => Err(e),
            Err(Err::Failure(e)) => Err(e),
        }
    }

    fn parser(input: &[u8]) -> Parser<Self> {
        let (input, _) = context("core magic bytes", parse_magic)(input)?;
        let (input, _) = context("core version", parse_core_version)(input)?;
        let (input, _) = context("game magic bytes", parse_game_magic::<W>)(input)?;
        let (input, _) = context("game version", parse_game_version::<W>)(input)?;
        let (input, rate) = context("simulation rate", be_u32)(input)?;
        let (input, initial) = context("initial world", length_decoding(ciborium_parse))(input)?;
        let (input, inputs) = context("inputs", decode_vec(parse_turn::<W>))(input)?;
        Ok((
            input,
            Replay {
                rate,
                initial: initial.unwrap_or_default(),
                inputs,
            },
        ))
    }
}

fn parse_magic(input: &[u8]) -> Parser<()> {
    let (input, magic) = take(4_u32)(input)?;
    if magic != MAGIC_BYTES {
        let mut magic_buff = [0; 4];
        magic_buff.copy_from_slice(magic);
        Err(Err::Failure(Error::InvalidMagic(magic_buff)))
    } else {
        Ok((input, ()))
    }
}

fn parse_game_magic<W: World>(input: &[u8]) -> Parser<()> {
    let (input, magic) = take(4_u32)(input)?;
    if magic != W::magic_bytes() {
        let mut magic_buff = [0; 4];
        magic_buff.copy_from_slice(magic);
        Err(Err::Failure(Error::InvalidMagic(magic_buff)))
    } else {
        Ok((input, ()))
    }
}

fn parse_core_version(input: &[u8]) -> Parser<u32> {
    let (input, version) = be_u32(input)?;
    if version != REPLAY_FORMAT_VERSION {
        Err(Err::Failure(Error::UnsupportedCoreVersion(version)))
    } else {
        Ok((input, version))
    }
}

fn parse_game_version<W: World>(input: &[u8]) -> Parser<u32> {
    let (input, version) = be_u32(input)?;
    if !W::guard_version(version) {
        Err(Err::Failure(Error::UnsupportedGameVersion(version)))
    } else {
        Ok((input, version))
    }
}

fn parse_turn<W: World>(input: &[u8]) -> Parser<(u64, Vec<W::Input>)> {
    let (input, turn) = context("turn number", be_u64)(input)?;
    let (input, inputs) = context("turn inputs", decode_vec(parse_input::<W>))(input)?;
    Ok((input, (turn, inputs)))
}

fn parse_input<W: World>(input: &[u8]) -> Parser<W::Input> {
    let (input, input_opt) = context("turn input", length_decoding(ciborium_parse))(input)?;
    if let Some(turn_input) = input_opt {
        Ok((input, turn_input))
    } else {
        Err(nom::Err::Failure(Error::MissingTurnInput))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::fmt::Debug;

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    struct TestWorld1 {}
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    enum TestInput1 {}

    impl World for TestWorld1 {
        type Input = TestInput1;

        fn magic_bytes() -> [u8; 4] {
            *b"TWD1"
        }

        fn current_version() -> u32 {
            1
        }
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    struct TestWorld2 {
        field1: u32,
    }
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    enum TestInput2 {
        Add(u32),
        Sub(u32),
    }

    impl World for TestWorld2 {
        type Input = TestInput2;

        fn magic_bytes() -> [u8; 4] {
            *b"TWD2"
        }

        fn current_version() -> u32 {
            1
        }
    }

    #[test]
    fn encode_decode_id() {
        env_logger::init();

        let replay1 = Replay::<TestWorld1>::new(&TestWorld1 {}, 60);
        make_encode_decode_test(replay1);

        let replay2 = Replay::<TestWorld2>::new(&TestWorld2 { field1: 42 }, 60);
        make_encode_decode_test(replay2);

        let mut replay3 = Replay::<TestWorld2>::new(&TestWorld2 { field1: 42 }, 60);
        replay3.record(0, &vec![]).expect("record");
        make_encode_decode_test(replay3);

        let mut replay4 = Replay::<TestWorld2>::new(&TestWorld2 { field1: 42 }, 60);
        replay4
            .record(1, &vec![TestInput2::Add(4)])
            .expect("record");
        make_encode_decode_test(replay4);

        let mut replay5 = Replay::<TestWorld2>::new(&TestWorld2 { field1: 42 }, 60);
        replay5
            .record(1, &vec![TestInput2::Add(4), TestInput2::Sub(2)])
            .expect("record");
        make_encode_decode_test(replay5);

        let mut replay6 = Replay::<TestWorld2>::new(&TestWorld2 { field1: 42 }, 60);
        replay6.record(0, &vec![]).expect("record");
        replay6
            .record(1, &vec![TestInput2::Add(4)])
            .expect("record");
        replay6
            .record(2, &vec![TestInput2::Sub(2), TestInput2::Add(8)])
            .expect("record");
        make_encode_decode_test(replay6);
    }

    #[test]
    fn save_load_test() {
        env_logger::init();

        let mut replay1 = Replay::<TestWorld2>::new(&TestWorld2 { field1: 42 }, 60);
        replay1.record(0, &vec![]).expect("record");
        replay1
            .record(1, &vec![TestInput2::Add(4)])
            .expect("record");
        replay1
            .record(2, &vec![TestInput2::Sub(2), TestInput2::Add(8)])
            .expect("record");
        make_save_load_test(replay1);
    }

    fn make_encode_decode_test<
        W: World + Clone + PartialEq + Default + Debug + Serialize + DeserializeOwned,
    >(
        replay: Replay<W>,
    ) {
        let mut buffer = vec![];
        replay.encode(&mut buffer).expect("encoded");
        let replay_decoded = Replay::<W>::decode(&buffer).expect("decoded");
        assert_eq!(replay, replay_decoded);
    }

    fn make_save_load_test<
        W: World + Clone + PartialEq + Default + Debug + Serialize + DeserializeOwned,
    >(
        replay: Replay<W>,
    ) {
        let t = temp_file::TempFile::new().expect("temp file");
        replay.save(t.path()).expect("save replay");
        let replay_loaded = Replay::<W>::load(t.path()).expect("load replay");
        assert_eq!(replay, replay_loaded);
    }
}
