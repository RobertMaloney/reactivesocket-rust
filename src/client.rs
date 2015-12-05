use mio::*;
use mio::tcp::TcpStream;
use std::str;
use std::sync::mpsc;
use std::net::SocketAddr;
use std::thread;
use std::fs::File;
use std::io::prelude::*;

const CLIENT_TOKEN: Token = Token(1);
const FILE_PATH: &'static str = "mioclient.log";

pub struct Client {
  pub socket: TcpStream,
  rx: mpsc::Receiver<String>,
  file: File
}

impl Client {
  pub fn new(address: SocketAddr) -> mpsc::Sender<String> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
      let socket = TcpStream::connect(&address).unwrap();

      let mut server = Client {
        socket: socket,
        rx: rx,
        file: File::create(FILE_PATH).unwrap()
      };
      let mut event_loop = EventLoop::new().unwrap();
      event_loop.register(&server.socket, CLIENT_TOKEN,
        EventSet::readable() | EventSet::writable(), PollOpt::level()).unwrap();
      event_loop.run(&mut server).unwrap();
    });

    tx
  }

  fn handle_events(&mut self, events: EventSet) {
    if events.is_readable() {
      self.read();
    }
    if events.is_writable() {
      self.write();
    }
  }

  fn read(&mut self) {
    debug!("Reading");
    loop {
      let mut buf = [0; 2048];
      match self.socket.try_read(&mut buf) {
          Err(e) => {
              // println!("Error while reading socket: {:?}", e);
              return
          },
          Ok(None) =>
              // Socket buffer has got no more bytes.
              break,
          Ok(Some(len)) => {
            // println!("{} bytes read.", len);
            let message = str::from_utf8(&buf[0..len]).unwrap();
            // println!("Message: {}", message);
            write!(self.file, "Received Message: {}\n\n", message);
            self.file.flush().unwrap();
          }
      }
      debug!("Looping");
    }
    debug!("Done Reading");
  }

  fn write(&mut self) {
    debug!("Writing");
    // let mut buf = "The quick brown fox jumped over the lazy dog.".to_string().into_bytes();
    // self.socket.try_write(&mut buf).unwrap();
    loop {
      match self.rx.try_recv() {
        Ok(msg) => {
          write!(self.file, "Sending message: {}", msg).unwrap();
          self.file.flush().unwrap();
          self.socket.try_write(&mut msg.as_bytes()).unwrap();
          ()
        },
        Err(_) => {
          // write!(self.file, "No message received this loop\n").unwrap();
          break
        }
      }
    }
    debug!("Done Writing");
  }
}

impl Handler for Client {
  type Timeout = ();
  type Message = ();

  fn ready(&mut self, event_loop: &mut EventLoop<Client>, token: Token, events: EventSet) {
    debug!("Handling");
    // write!(self.file, "EVENTS: R: {}, W: {}\n", events.is_readable(), events.is_writable());
    self.handle_events(events);
  }
}
