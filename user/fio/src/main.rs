use std::{
    fs::OpenOptions,
    io::{Read, Seek, Write},
    path::Path,
};

fn main() {
    let path = Path::new("/dev/rnullb0");
    if !path.exists() {
        println!("The path {:?} does not exist", path);
    }
    let mut block = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    let mut buf = [0u8; 512];
    buf.fill(1);
    let w = block.write(&buf);
    println!("write: {:?}", w);
    block
        .seek(std::io::SeekFrom::Start(0))
        .expect("seek failed");
    let r = block.read(&mut buf);
    println!("read: {:?}", r);
    println!("buf: {:?}", buf);
}
