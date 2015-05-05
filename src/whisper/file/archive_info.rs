use std::io::{ Cursor, SeekFrom  };
use byteorder::{ BigEndian, ReadBytesExt, ByteOrder };

use whisper::point::{Point, POINT_SIZE};

#[derive(PartialEq,Debug)]
pub struct ArchiveInfo {
    pub offset: u64,
    pub seconds_per_point: u64,
    pub points: u64,
    pub retention: u64,
    size_in_bytes: u64
}

pub fn slice_to_archive_info(buf: &[u8]) -> ArchiveInfo {
    let mut cursor = Cursor::new(buf);
    let offset = cursor.read_u32::<BigEndian>().unwrap();
    let seconds_per_point = cursor.read_u32::<BigEndian>().unwrap();
    let points = cursor.read_u32::<BigEndian>().unwrap();

    let point_size = POINT_SIZE as u32;
    let size_in_bytes = (seconds_per_point * points * point_size) as u64;

    ArchiveInfo {
        offset: offset as u64,
        seconds_per_point: seconds_per_point as u64,
        points: points as u64,
        retention: (seconds_per_point * points) as u64,
        size_in_bytes: size_in_bytes
    }
}

impl ArchiveInfo {
    pub fn calculate_seek(&self, point: &Point, base_timestamp: u64) -> SeekFrom {
        if base_timestamp == 0 {
            return SeekFrom::Start(0);
        } else {

            let file_offset = {
                let time_since_base_time = (point.timestamp - base_timestamp) as u64;
                let points_away_from_base_time = time_since_base_time / self.seconds_per_point;
                let point_size = POINT_SIZE as u64;
                let bytes_away_from_offset = (points_away_from_base_time * point_size) as u64;
                self.offset + (bytes_away_from_offset % (self.size_in_bytes))
            };

            return SeekFrom::Start(file_offset);
        }
    }

    pub fn interval_ceiling(&self, point: &Point) -> u64 {
        point.timestamp - (point.timestamp % self.seconds_per_point)
    }
}
