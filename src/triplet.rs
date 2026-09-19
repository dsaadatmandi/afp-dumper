// MO:DCA triplets: one length byte (counting itself), one id byte, then body.
// only TLE is triplet-based — NOP's data is UndfData, free-form bytes with no
// architectural definition (MO:DCA Reference p.299), so it never comes here

pub const T_FQN: u8 = 0x02;
pub const T_ATTR_VALUE: u8 = 0x36;

// the FQN type that carries a TLE's attribute name (p.342)
pub const FQN_ATTRIBUTE_NAME: u8 = 0x0B;

#[derive(Debug, Clone, Copy)]
pub struct Triplet<'a> {
    pub id: u8,
    pub body: &'a [u8],
}

pub struct TripletIter<'a> {
    data: &'a [u8],
    pos: usize,
}

pub fn triplets(data: &[u8]) -> TripletIter<'_> {
    TripletIter { data, pos: 0 }
}

impl<'a> Iterator for TripletIter<'a> {
    type Item = Triplet<'a>;

    fn next(&mut self) -> Option<Triplet<'a>> {
        let rest = self.data.get(self.pos..)?;
        if rest.len() < 2 {
            return None;
        }
        let len = rest[0] as usize;
        // a length below 2 or past the end means the data is not really a
        // triplet sequence, stop rather than resync into garbage
        if len < 2 || len > rest.len() {
            return None;
        }
        self.pos += len;
        Some(Triplet {
            id: rest[1],
            body: &rest[2..len],
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Fqn<'a> {
    pub fqn_type: u8,
    pub name: &'a [u8],
}

impl<'a> Triplet<'a> {
    pub fn as_fqn(&self) -> Option<Fqn<'a>> {
        if self.body.len() < 2 {
            return None;
        }
        // body[1] is FQNFmt, which describes the name rather than carrying it
        Some(Fqn {
            fqn_type: self.body[0],
            name: &self.body[2..],
        })
    }

    // X'36' is two reserved bytes then the value, which may be absent
    // entirely — that means a null attribute value, not a malformed triplet
    pub fn as_attr_value(&self) -> Option<&'a [u8]> {
        if self.body.len() < 2 {
            return None;
        }
        Some(&self.body[2..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triplet(id: u8, body: &[u8]) -> Vec<u8> {
        let mut v = vec![(body.len() + 2) as u8, id];
        v.extend_from_slice(body);
        v
    }

    #[test]
    fn walks_a_tle_triplet_sequence() {
        // X'01' declares a code page and X'80' is an attribute qualifier —
        // both describe the field rather than carry its content, so the walker
        // steps over them and only the name and value are reachable
        let mut data = triplet(0x01, &[0xff, 0xff, 0x01, 0xf4]);
        data.extend(triplet(T_FQN, &[FQN_ATTRIBUTE_NAME, 0x00, b'A', b'B']));
        data.extend(triplet(T_ATTR_VALUE, &[0x00, 0x00, b'1']));
        data.extend(triplet(0x80, &[0, 0, 0, 7, 0, 0, 0, 1]));

        let found: Vec<_> = triplets(&data).collect();
        assert_eq!(found.len(), 4);

        let fqn = found[1].as_fqn().unwrap();
        assert_eq!(fqn.fqn_type, FQN_ATTRIBUTE_NAME);
        assert_eq!(fqn.name, b"AB");

        assert_eq!(found[2].as_attr_value().unwrap(), b"1");
    }

    #[test]
    fn null_attribute_value_is_not_an_error() {
        let data = triplet(T_ATTR_VALUE, &[0x00, 0x00]);
        let t = triplets(&data).next().unwrap();
        assert_eq!(t.as_attr_value().unwrap(), b"");
    }

    #[test]
    fn stops_on_malformed_lengths() {
        // length 0 and length past the end both terminate the walk
        assert_eq!(triplets(&[0x00, 0x01, 0x02]).count(), 0);
        assert_eq!(triplets(&[0xf0, 0x01, 0x02]).count(), 0);
        // a good triplet followed by a bad one yields only the good one
        let mut data = triplet(T_FQN, &[0x0b, 0x00, b'X']);
        data.extend_from_slice(&[0xf0, 0x36]);
        assert_eq!(triplets(&data).count(), 1);
    }

}
