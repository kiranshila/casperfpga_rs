//! Pure-rust implementation of Dave's "Compact Sorted List"

use std::str::Utf8Error;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Error while parsing CSL bytes")]
    Parse,
    #[error(transparent)]
    Utf8(#[from] Utf8Error),
}

/// Read a CSL from bytes
///
/// # Errors
/// Returns errors on invalid CSL
pub fn from_bytes(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, Error> {
    let mut v = vec![];
    // First byte specifies the size of the fixed-length payload
    let payload_n = usize::from(*bytes.first().ok_or(Error::Parse)?);
    let mut ptr = 1;
    // Grab the first string and payload
    let key_n = usize::from(*bytes.get(ptr).ok_or(Error::Parse)?);
    ptr += 1;
    let key_bytes = bytes.get(ptr..(key_n + ptr)).ok_or(Error::Parse)?;
    ptr += key_n;
    let key_string = std::str::from_utf8(key_bytes)?;
    let key_pl = bytes
        .get(ptr..(payload_n + ptr))
        .ok_or(Error::Parse)?
        .to_vec();
    ptr += payload_n;
    v.push((key_string.to_string(), key_pl));

    // Now read the rest
    loop {
        let header_n = usize::from(*bytes.get(ptr).ok_or(Error::Parse)?);
        ptr += 1;
        let tail_n = usize::from(*bytes.get(ptr).ok_or(Error::Parse)?);
        ptr += 1;
        // Check end condition
        if header_n == 0 && tail_n == 0 {
            break;
        }
        // Pull out `header_n` chars from previous string and append to `tail_n` chars next
        let head = &v.last().ok_or(Error::Parse)?.0[..header_n];
        let tail_bytes = bytes.get(ptr..(tail_n + ptr)).ok_or(Error::Parse)?;
        let tail = std::str::from_utf8(tail_bytes)?;
        let key = format!("{head}{tail}");
        ptr += tail_n;
        // Pull out payload
        let payload = bytes
            .get(ptr..(payload_n + ptr))
            .ok_or(Error::Parse)?
            .to_vec();
        ptr += payload_n;
        // Push
        v.push((key, payload));
    }

    Ok(v)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_from_csl() {
        let csl = [
            0x01, 0x0D, b'a', b'd', b'c', b'1', b'6', b'_', b'w', b'b', b'_', b'r', b'a', b'm',
            b'1', 0x01, 0x0C, 0x01, b'2', 0x02, 0x00, 0x09, b'e', b'q', b'_', b'0', b'_', b'g',
            b'a', b'i', b'n', 0x03, 0x03, 0x06, b'1', b'_', b'g', b'a', b'i', b'n', 0x04, 0x01,
            0x0C, b't', b'h', b'_', b'0', b'_', b'b', b'f', b'r', b'a', b'm', b'e', b's', 0x05,
            0x06, 0x04, b'c', b'o', b'r', b'e', 0x06, 0x00, 0x00,
        ];
        let unpacked = from_bytes(&csl).unwrap();
        assert_eq!(
            unpacked,
            vec![
                ("adc16_wb_ram1".to_string(), vec![0x01]),
                ("adc16_wb_ram2".to_string(), vec![0x02]),
                ("eq_0_gain".to_string(), vec![0x03]),
                ("eq_1_gain".to_string(), vec![0x04]),
                ("eth_0_bframes".to_string(), vec![0x05]),
                ("eth_0_core".to_string(), vec![0x06])
            ]
        );
    }
}
