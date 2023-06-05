extern crate base64;
extern crate hex;
extern crate httparse;

use crypto::digest::Digest;
use crypto::sha1::Sha1;
use httparse::Header;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    println!("Listening for connections on port {}", 8080);

    for stream in listener.incoming() {
        thread::spawn(move || {
            let s = stream.unwrap();
            handle_client(&s);
        });
    }
}

fn handle_client(mut stream: &TcpStream) {
    let mut buf = [0; 4096];
    stream.read(&mut buf).unwrap();
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut req = httparse::Request::new(&mut headers);
    req.parse(&buf).unwrap();
    let path = req.path.expect("fail");
    match path {
        "/" => {
            let res = generate_connect_websocket_response();
            let data = res.as_bytes();
            stream.write(data).unwrap();
        }
        "/websocket" => {
            let sha1_base64 = generate_sec_websocket_accept(req.headers);
            let header = create_response_header(sha1_base64);
            let data = header.as_bytes();
            stream.write(data).unwrap();

            loop {
                let mut msg_buf = [0; 1024];

                if stream.read(&mut msg_buf).is_ok() {
                    let opcode = msg_buf[0] & 0x0f;

                    if opcode == 1 {
                        let payload_length = (msg_buf[1] & 0b1111110) as usize;
                        let mask_key = &msg_buf[2..6];
                        let mut payload = vec![0u8; payload_length];

                        for (i, byte) in msg_buf[6..(6 + payload_length)].iter().enumerate() {
                            payload[i] = byte ^ mask_key[i % 4];
                        }

                        let payload = String::from_utf8(payload).unwrap();
                        println!("{}", payload);
                        stream.write(&[129, 5, 72, 101, 108, 108, 111]).unwrap();
                    }
                } else {
                    break;
                }
            }
        }
        _ => {}
    }
}

fn generate_connect_websocket_response() -> String {
    let client_code = "function () {
        var ws = new WebSocket('ws://localhost:8080/websocket', ['test', 'chat']);
        ws.onopen = function() {
            console.log(ws);
            ws.send('test');
            ws.onmessage = function(message) {
                console.log(message.data);
            };
        }
    }";
    let body = "<html><head><title>rust web socket</title><script type='text/javascript'>("
        .to_string()
        + client_code
        + ")()</script></head><body>hello world!!!!!</body></html>";
    let status = "HTTP/1.1 200 OK\r\n".to_string();
    let header = status + "Content-Type: text/html; charset=UTF-8\r\n\r\n";
    return header + &body + "\r\n";
}

fn generate_sec_websocket_accept(header: &mut [Header]) -> String {
    let token_bytes = header
        .iter()
        .find(|&&x| x.name == "Sec-WebSocket-Key")
        .unwrap()
        .value;
    let token_bytes_str = std::str::from_utf8(token_bytes).unwrap();

    // Merge token and key
    let key = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let joined_token = &*(token_bytes_str.to_string() + key);

    // Hash SHA-1
    let mut hasher = Sha1::new();
    hasher.input(joined_token.as_bytes());
    let sha1_string = hasher.result_str();

    //Encode Base64
    let bytes = hex::decode(sha1_string).unwrap();
    return base64::encode(bytes);
}

fn create_response_header(sha1_base64: String) -> String {
    let status = "HTTP/1.1 101 Switching Protocols\r\n".to_string();
    let header = status
        + "Upgrade: websocket\r\n"
        + "Connection: Upgrade\r\n"
        + "Sec-WebSocket-Accept: "
        + &*sha1_base64
        + "\r\n"
        + "Sec-WebSocket-Protocol: chat\r\n\r\n";
    return header;
}
