use mio::*;
use mio::tcp::{TcpListener, TcpStream};
use std::collections::HashMap;
use std::str;
use std::sync::mpsc;
use std::net::SocketAddr;
use std::thread;

const SERVER_TOKEN: Token = Token(0);

struct Client {
  socket: TcpStream,
  interest: EventSet,
  outgoing: Vec<String>
}

impl Client {
  fn new(socket: TcpStream, interest: EventSet) -> Client {
    Client {
      socket: socket,
      interest: interest,
      outgoing: Vec::new()
    }
  }
}

pub struct Server {
  socket: TcpListener,
  clients: HashMap<Token, Client>,
  token_counter: usize,
  tx: mpsc::Sender<String>
}

impl Server {
  pub fn new(address: SocketAddr) -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
      let socket = TcpListener::bind(&address).unwrap();

      let mut server = Server {
        socket: socket,
        clients: HashMap::new(),
        token_counter: 1,
        tx: tx
      };
      let mut event_loop = EventLoop::new().unwrap();
      event_loop.register(&server.socket, SERVER_TOKEN,
        EventSet::readable(), PollOpt::edge()).unwrap();
      event_loop.run(&mut server).unwrap();
    });

    rx
  }
}

impl Handler for Server {
    type Timeout = ();
    type Message = ();

    fn ready(&mut self, event_loop: &mut EventLoop<Server>, token: Token, events: EventSet) {
        debug!("{:?} -> R: {} W: {} E: {} H: {}", token, events.is_readable(), events.is_writable(),
          events.is_error(), events.is_hup());
        match token {
            SERVER_TOKEN => {
              debug!("SERVER");
              debug!("Accepting new client");
              let client_socket = match self.socket.accept() {
                    Err(e) => {
                        println!("Accept error: {}", e);
                        return;
                    },
                    Ok(None) => unreachable!("Accept has returned 'None'"),
                    Ok(Some((sock, _))) => sock
                };

                // Have to make this atomic
                self.token_counter += 1;
                let new_token = Token(self.token_counter);

                self.clients.insert(new_token, Client::new(client_socket, EventSet::readable()));
                event_loop.register(&self.clients[&new_token].socket, new_token,
                  self.clients[&new_token].interest, PollOpt::edge() | PollOpt::oneshot()).unwrap();
            },
            token => {
              let mut client = self.clients.get_mut(&token).unwrap();
              if events.is_readable() {
                let mut buf = [0; 1024];
                match client.socket.try_read(&mut buf) {
                  Err(e) => {
                    println!("Error while reading socket: {:?}", e);
                    return
                  },
                  Ok(None) => (),
                  // Socket buffer has got no more bytes.
                  Ok(Some(len)) => {
                    debug!("Read {} bytes from client {:?}", len, token);
                    if len > 0 {
                      client.outgoing.push(str::from_utf8(&buf[0..len]).unwrap().to_string());
                    }
                  }
                }
                client.interest = EventSet::writable();
                event_loop.reregister(&client.socket, token, client.interest,
                  PollOpt::edge() | PollOpt::oneshot()).unwrap();
              }

              if events.is_writable() {
                for msg in client.outgoing.iter() {
                  let mut buf = msg.as_bytes();
                  client.socket.try_write(&mut buf).unwrap();
                }
                client.outgoing.clear();

                client.interest = EventSet::readable();
                event_loop.reregister(&client.socket, token, client.interest,
                  PollOpt::edge() | PollOpt::oneshot()).unwrap();
              }
            }
        }
    }
}
