use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

// structured field introducer: SFLength(2) + SFTypeID(3) + FlagByte(1) +
// Reserved(2). SFLength counts itself but not the X'5A' framing byte
// (MO:DCA Reference p.20-21)
const SFI_LEN: usize = 8;
const MIN_SF_LEN: u16 = 8;

const FLAG_EXTENSION: u8 = 0x80; // bit 0 — SFI extension follows the introducer
const FLAG_PADDING: u8 = 0x08; // bit 4 — padding appended after the data

const FRAME_BYTE: u8 = 0x5a;

#[derive(Debug)]
pub struct SfRecord {
    pub type_id: [u8; 3],
    pub data: Vec<u8>,
}

// reads whole structured fields at offsets the aho-corasick pass already
// found. deliberately not a full-file record walker: on a multi-gigabyte file
// only the handful of NOP/TLE hits are ever seeked to and decoded
pub struct RecordReader {
    file: File,
    file_size: u64,
    framed: bool,
}

impl RecordReader {
    pub fn open(path: &Path) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let file_size = file.metadata()?.len();

        // the X'5A' prefix is platform framing, not architecture (p.21), so
        // sniff it once instead of assuming either layout
        let mut first = [0u8; 1];
        let framed = file.read_exact(&mut first).is_ok() && first[0] == FRAME_BYTE;

        Ok(Self {
            file,
            file_size,
            framed,
        })
    }

    // id_offset points at the 3-byte SFTypeID, which is where the pattern
    // matched. returns None when the surrounding bytes do not look like a real
    // record — the same byte sequence can occur inside image or text payloads
    pub fn read_at(&mut self, id_offset: u64) -> io::Result<Option<SfRecord>> {
        // SFLength always sits immediately before the type id
        let Some(sfi_start) = id_offset.checked_sub(2) else {
            return Ok(None);
        };
        let record_start = if self.framed {
            match sfi_start.checked_sub(1) {
                Some(s) => s,
                None => return Ok(None),
            }
        } else {
            sfi_start
        };

        if self.framed {
            let mut frame = [0u8; 1];
            self.read_exact_at(record_start, &mut frame)?;
            if frame[0] != FRAME_BYTE {
                return Ok(None);
            }
        }

        let mut sfi = [0u8; SFI_LEN];
        if sfi_start + SFI_LEN as u64 > self.file_size {
            return Ok(None);
        }
        self.read_exact_at(sfi_start, &mut sfi)?;

        let sf_len = u16::from_be_bytes([sfi[0], sfi[1]]);
        if sf_len < MIN_SF_LEN {
            return Ok(None);
        }

        let record_end = sfi_start + sf_len as u64;
        if record_end > self.file_size {
            return Ok(None);
        }

        // strongest cheap check: a genuine record is followed by the next
        // record's framing byte, or by end of file
        if self.framed && record_end < self.file_size {
            let mut next = [0u8; 1];
            self.read_exact_at(record_end, &mut next)?;
            if next[0] != FRAME_BYTE {
                return Ok(None);
            }
        }

        let flags = sfi[5];
        let mut data_start = sfi_start + SFI_LEN as u64;

        // the optional SFI extension sits between the introducer and the data,
        // and its first byte is its own length (p.21)
        if flags & FLAG_EXTENSION != 0 {
            if data_start >= record_end {
                return Ok(None);
            }
            let mut ext = [0u8; 1];
            self.read_exact_at(data_start, &mut ext)?;
            data_start += ext[0].max(1) as u64;
        }

        if data_start > record_end {
            return Ok(None);
        }

        let mut data = vec![0u8; (record_end - data_start) as usize];
        self.read_exact_at(data_start, &mut data)?;

        if flags & FLAG_PADDING != 0 {
            strip_padding(&mut data);
        }

        Ok(Some(SfRecord {
            type_id: [sfi[2], sfi[3], sfi[4]],
            data,
        }))
    }

    fn read_exact_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<()> {
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read_exact(buf)
    }
}

// padding length lives in the last padding byte, or — when that byte is X'00'
// — in the two bytes before it (p.24)
fn strip_padding(data: &mut Vec<u8>) {
    let n = data.len();
    if n == 0 {
        return;
    }

    let pad = if data[n - 1] == 0x00 {
        if n < 3 {
            return;
        }
        u16::from_be_bytes([data[n - 3], data[n - 2]]) as usize
    } else {
        data[n - 1] as usize
    };

    if pad > 0 && pad <= n {
        data.truncate(n - pad);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn sf(type_id: &[u8; 3], data: &[u8], flags: u8) -> Vec<u8> {
        let len = (SFI_LEN + data.len()) as u16;
        let mut v = vec![FRAME_BYTE];
        v.extend_from_slice(&len.to_be_bytes());
        v.extend_from_slice(type_id);
        v.push(flags);
        v.extend_from_slice(&[0x00, 0x00]);
        v.extend_from_slice(data);
        v
    }

    fn write_temp(bytes: &[u8], name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(name);
        File::create(&path).unwrap().write_all(bytes).unwrap();
        path
    }

    const NOP: [u8; 3] = [0xd3, 0xee, 0xee];

    #[test]
    fn reads_record_data_at_a_hit_offset() {
        let blob = sf(&NOP, b"hello", 0x00);
        let path = write_temp(&blob, "afp_record_basic.afp");
        let mut r = RecordReader::open(&path).unwrap();

        // offset 3 is where the type id sits behind the X'5A' frame and the
        // two length bytes
        let rec = r.read_at(3).unwrap().unwrap();
        assert_eq!(rec.type_id, NOP);
        assert_eq!(rec.data, b"hello");
    }

    #[test]
    fn rejects_a_hit_inside_payload_bytes() {
        // the NOP id appearing in the middle of another record's data must not
        // decode as a record of its own
        let blob = sf(&[0xd3, 0xee, 0xfb], b"\x00\x08\xd3\xee\xee\x00\x00\x00junk", 0x00);
        let path = write_temp(&blob, "afp_record_falsepos.afp");
        let mut r = RecordReader::open(&path).unwrap();

        let hit = blob
            .windows(3)
            .position(|w| w == NOP)
            .expect("planted id") as u64;
        assert!(r.read_at(hit).unwrap().is_none());
    }

    #[test]
    fn strips_trailing_padding() {
        // one byte of padding, length carried in that byte
        let blob = sf(&NOP, b"data\x01", FLAG_PADDING);
        let path = write_temp(&blob, "afp_record_pad.afp");
        let mut r = RecordReader::open(&path).unwrap();
        assert_eq!(r.read_at(3).unwrap().unwrap().data, b"data");
    }

    #[test]
    fn strips_long_padding_form() {
        // four pad bytes, length in the last three with a X'00' terminator
        let blob = sf(&NOP, b"data\x00\x00\x04\x00", FLAG_PADDING);
        let path = write_temp(&blob, "afp_record_pad_long.afp");
        let mut r = RecordReader::open(&path).unwrap();
        assert_eq!(r.read_at(3).unwrap().unwrap().data, b"data");
    }

    #[test]
    fn skips_an_sfi_extension() {
        // extension is 3 bytes including its own length byte
        let blob = sf(&NOP, b"\x03\xaa\xbbreal", FLAG_EXTENSION);
        let path = write_temp(&blob, "afp_record_ext.afp");
        let mut r = RecordReader::open(&path).unwrap();
        assert_eq!(r.read_at(3).unwrap().unwrap().data, b"real");
    }

    #[test]
    fn rejects_a_length_running_past_end_of_file() {
        let mut blob = sf(&NOP, b"hello", 0x00);
        blob[1] = 0x7f; // absurd SFLength
        blob[2] = 0xff;
        let path = write_temp(&blob, "afp_record_overrun.afp");
        let mut r = RecordReader::open(&path).unwrap();
        assert!(r.read_at(3).unwrap().is_none());
    }
}
