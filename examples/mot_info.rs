use anyhow::*;
use bvh_anim::*;
use mot::*;
use structopt::StructOpt;

use std::path::PathBuf;

#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
struct Opt {
    /// Input file
    #[structopt(parse(from_os_str))]
    input: PathBuf,

    #[structopt(parse(from_os_str))]
    bone_db: PathBuf,
}

use std::fs::File;
use std::io::{self, Read};

use mot::read::DeserializeEndian;
use nom::number::Endianness;

use cookie_factory::*;

use log::*;
use env_logger::*;

use diva_db::mot::*;

fn main() -> Result<()> {
    let env = Env::default()
        .filter_or("MY_LOG_LEVEL", "trace")
        .write_style_or("MY_LOG_STYLE", "always");

    env_logger::init_from_env(env);

    info!("starting up");

    let opt = Opt::from_args();
    let mut file = File::open(&opt.input)?;
    let mut data = vec![];
    file.read_to_end(&mut data)?;

    let (_, mot) = Motion::parse(&data, Endianness::Little).unwrap();

    let mut file = File::open(&opt.bone_db)?;
    let mut data = vec![];
    file.read_to_end(&mut data)?;

    let (_, motset_db) = MotionSetDatabase::read(Endianness::Little)(&data).unwrap();

    for id in mot.bones {
        println!("{}: {}", id, motset_db.bones[id]);
    }

    Ok(())
}
