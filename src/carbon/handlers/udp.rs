use whisper::{ NamedPoint };

use std::thread::{ self, JoinHandle };
use std::net::UdpSocket;
use std::io::Error;
use std::sync::mpsc::{ SyncSender };

use super::super::Config;
use super::Action;

pub fn run_server<'a>(tx: SyncSender<Action>, config: &Config) -> Result<JoinHandle<()>,Error> {
    info!("UDP server binding to `{}`", config.bind_spec);
    let mut buf_box = create_buffer();
    let socket = try!( UdpSocket::bind(config.bind_spec) );

    let join_handle = thread::spawn(move ||{
        loop {
            let (bytes_read,_) = {
                // debug!("waiting on recv from socket");

                match socket.recv_from( &mut buf_box[..] ) {
                    Ok(res) => {
                        res
                    },
                    Err(err) => {
                        error!("error reading from socket: {:?}", err);
                        continue;
                    }
                }
            };

            debug!("parsing point...");

            match NamedPoint::from_datagram(&buf_box[0..bytes_read]) {
                Ok(named_points) => {
                    // Dies if the receiver is closed
                    debug!("putting message on tx");
                    for named_point in named_points {
                        tx.send(Action::Write(named_point)).unwrap();
                    }
                },
                Err(err) => {
                    debug!("wtf mate: {:?}", err);
                }
            };
        }
    });
    Ok(join_handle)
}

fn create_buffer() -> Box<[u8]> {
    let buf : [u8; 8*1024] = [0; 8*1024];
    Box::new( buf )
}
