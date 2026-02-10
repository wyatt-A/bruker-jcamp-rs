use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use jcamp_rs::{parse_paravision_params, PvAtom, PvValue};

fn main() {

    let f = File::open("/Users/wyatt/20260209_143522_B26020901_25_abb_12_1_1/8/acqp").unwrap();
    let pv = parse_paravision_params(BufReader::new(f)).unwrap();

    let (n_read,n_proj):(usize,usize) = if let PvValue::Array {items,..} = pv.params.get("ACQ_size").unwrap() {
        (items[0].clone().into(),items[1].clone().into())
    }else {
        panic!("failed to parse ACQ_size");
    };

    // this is the number of frames
    let n_frames:usize = if let PvValue::Scalar(nr) = pv.params.get("NR").unwrap() {
        nr.clone().into()
    }else {
        panic!("failed to parse number of frames");
    };

    let n_coils:usize = if let PvValue::Array{items,..} = pv.params.get("ACQ_ReceiverSelect").unwrap() {
        items.iter()
            .filter(|v| matches!(v, PvAtom::Bool(true)))
            .count()
    }else {
        panic!("failed to parse number of receivers");
    };

    println!("n_coils:{}",n_coils);
    println!("n_frames:{}",n_frames);
    println!("n_read:{}",n_read);
    println!("n_proj:{}",n_proj);


    // let val = pv.params.get("ACQ_dim").unwrap().to_owned();
    // println!("{:?}", val)
}