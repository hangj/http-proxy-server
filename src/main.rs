use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:1080").await.unwrap();

    loop {
        let (mut stream, _addr) = listener.accept().await.unwrap();
        stream.set_nodelay(true).unwrap();

        tokio::spawn(async move {
            let (reader, mut writer) = stream.split();

            let mut reader = BufReader::new(reader);

            let mut line = String::with_capacity(64);

            // "CONNECT hangj.cnblogs.com:443 HTTP/1.1\r\n"
            // "GET http://hangj.cnblogs.com/ HTTP/1.1\r\n"
            reader.read_line(&mut line).await.unwrap();
            let vec = line.split(char::is_whitespace).filter(|s|!s.is_empty()).collect::<Vec<_>>();
            assert_eq!(vec.len(), 3);

            let method = vec[0];
            let uri = vec[1];
            let version = vec[2];

            // find host, port and path
            let (host, port, path) = {
                let mut port = None;
                let h_uri = if let Some(uri) = uri.strip_prefix("http://") {
                    port = Some(80);
                    uri
                } else if let Some(uri) = uri.strip_prefix("https://") {
                    port = Some(443);
                    uri
                } else {
                    uri
                };

                let idx = h_uri.find('/').unwrap_or(h_uri.len());
                let host_port = &h_uri[..idx];
                let path = &h_uri[idx..];

                let host = if let Some(idx) = host_port.find(':') {
                    port = Some(host_port[idx + 1..].parse::<u16>().unwrap());

                    &host_port[..idx]
                } else {
                    host_port
                };

                let Some(port) = port else {
                    eprintln!("Invalid uri: {uri}");
                    return;
                };
                (host, port, path)
            };

            println!("host: {host}, port: {port}, path: {path}");

            let mut remote_stream = TcpStream::connect((host, port)).await.unwrap();
            remote_stream.set_nodelay(true).unwrap();

            if method.eq_ignore_ascii_case("CONNECT") {
                writer.write_all(version.as_bytes()).await.unwrap();
                writer.write_all(b" 200 Connection Established\r\n\r\n").await.unwrap();
            } else {
                remote_stream.write_all(format!("{method} {path} {version}\r\n").as_bytes()).await.unwrap();
                remote_stream.write_all(reader.buffer()).await.unwrap();
            }

            tokio::io::copy_bidirectional(&mut stream, &mut remote_stream).await.unwrap();
        });
    }
}
