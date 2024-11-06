use async_nats::jetstream::Message;
use database::DbManager;
use exif::Reader;
use log::error;
use s3::Bucket;
use std::str;
use std::io::Cursor;

pub async fn handle_request(msg: Message, bucket: Box<Bucket>, db: DbManager) {
    let payload_bytes: &[u8] = &msg.payload;
    let source_media_id = match str::from_utf8(payload_bytes) {
        Ok(path) => path.to_owned(),
        Err(err) => {
            error!("Couldn't convert media path into utf8: {err:?}");
            return;
        }
    };

    let source_media_response = match bucket.get_object(source_media_id.clone()).await {
        Ok(oir) => oir,
        Err(err) => {
            error!("Get object failed: {err}");
            return;
        }
    };

    let source_media_bytes = source_media_response.bytes();
    let mut bufreader = Cursor::new(source_media_bytes);
    let exifreader = Reader::new();
    match exifreader.read_from_container(&mut bufreader) {
        Ok(exifdata) => {
            for f in exifdata.fields() {
                println!(
                    "{} {} {}",
                    f.tag,
                    f.ifd_num,
                    f.display_value().with_unit(&exifdata)
                );
            }
        }
        Err(..) => panic!("Error reading exif"),
    };
}
