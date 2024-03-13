use std::io;

pub trait Serialize {
    fn serialize<W: io::Write>(&self, writer: &mut W) -> io::Result<()>;

    fn to_bytes(&self) -> Vec<u8> {
        let mut output = Vec::new();
        self.serialize(&mut output).unwrap();
        output
    }
}
