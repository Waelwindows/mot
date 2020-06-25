use anyhow::*;
use bvh_anim::*;
use mot::*;
use structopt::StructOpt;
use slab_tree::*;

use std::path::PathBuf;

#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
struct Opt {
    /// Input file
    #[structopt(parse(from_os_str))]
    input: PathBuf,

    #[structopt(parse(from_os_str))]
    mot_db: PathBuf,

    #[structopt(parse(from_os_str))]
    bone_db: PathBuf,

    filter: Option<usize>,
}

use std::fs::File;
use std::io::{self, Read};

use mot::read::DeserializeEndian;
use nom::number::Endianness;

use cookie_factory::*;

use log::*;
use env_logger::*;

use diva_db::mot::*;
use diva_db::bone::*;

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

    let mut file = File::open(&opt.mot_db)?;
    let mut data = vec![];
    file.read_to_end(&mut data)?;

    let (_, motset_db) = MotionSetDatabase::read(Endianness::Little)(&data).unwrap();

    let mut file = File::open(&opt.bone_db)?;
    let mut data = vec![];
    file.read_to_end(&mut data)?;

    let (_, bone_db) = BoneDatabase::read(&data).unwrap();
    let skel = &bone_db.skeletons[0];

    for (i, id) in mot.bones.iter().enumerate() {
        let name = &motset_db.bones[*id];
        // let bone_id = skel.bones.iter().position(|x| &x.name == name);
        let bone = skel.bones.iter().find(|x| &x.name == name);
        let bone_pos = skel.bones.iter().position(|x| &x.name == name);
        // println!("{}#: mot id {}: {}",i, id, motset_db.bones[*id]);
        println!("{:03}: motid {:03} bonedb {:?} {:?}", i, id, bone_pos, bone);
    }
    println!("-----------------------------");
    for (i, bone) in skel.bones.iter().enumerate() {
        println!("{:03}: {:?}", i, bone);
    }
    println!("-----------------------------");
    for (i, bone) in skel.bones.iter().enumerate().filter(|(_, x)| x.unk2 != 0 && !x.name.contains("_r_") ) {
        println!("{:03}: {:?}", i, bone);
    }
    println!("-----------------------------");
    let mut tree = TreeBuilder::new().with_root(&skel.bones[0].name).with_capacity(skel.bones.len()).build();
    for bone in skel.bones.iter().skip(1) {
        let parent = match bone.parent.and_then(|x| skel.bones.get(x as usize)) {
            Some(n) => n,
            None => {warn!("bone {} doesn't have a parent", bone.name); continue}
        };
        let mut s = String::new();
        tree.write_formatted(&mut s).unwrap();
        // error!("\n{}", s);
        // debug!("{:?}", parent);
        // debug!("{:?}", bone);
        // let parent = skel.bones.get(jk)
        if bone.parent == Some(0) {
            // warn!("found 2nd root bone");
            let mut val = tree.root_mut().unwrap();
            val.append(&bone.name);
        } else {
            let id = tree.root().unwrap().traverse_pre_order().find(|x| &x.data()[..] == &parent.name[..]).map(|x| x.node_id());
            let mut val = match id.and_then(|id| tree.get_mut(id)) {
                Some(n) => n,
                None => { warn!("couldn't find parent {} for {}", parent.name, bone.name); continue }
            };
            val.append(&bone.name);
        }
    }
    // let mut s = String::new();
    // tree.write_formatted(&mut s).unwrap();
    // println!("{}", s);
    // dbg!(&mot.bones);
    // println!("-----------------------------");
    // for (i, id) in mot.bones.iter().enumerate() {
    //     let name = &skel.motion_bone_names.get(*id);
    //     println!("{}: motid {} {:?}", i, id, name)
    // }

    // let mut bonesets: Vec<(usize, &[FrameData])> = mot.bones.into_iter().zip(mot.sets.chunks(3)).collect();
    // bonesets.sort_by(|(i, _), (j, _)| i.cmp(j));
    // let (bones, sets): (Vec<usize>, Vec<&[FrameData]>) = bonesets.into_iter().unzip();
    // let sets = sets.into_iter().flat_map(|x| x).cloned().collect();

    // let mot = Motion { bones, sets };

    let mut file = File::create("/home/waelwindows/rust/mot/spoopy.mot")?;
    mot.write()(&mut file)?;

    Ok(())
}
