use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpSocket, TcpStream},
    sync::broadcast::{self, error::RecvError},
};

#[tokio::main]
async fn main() {

    // let listener = TcpListener::bind("127.0.0.1:8888").unwrap();
    let listener = {
        let addr = "127.0.0.1:1080";
        let backlog = 1024;

        let socket = TcpSocket::new_v4().unwrap();
        socket.bind(addr.parse().unwrap()).unwrap();
        socket.set_reuseaddr(true).unwrap();
        socket.set_reuseport(true).unwrap();
        socket.set_nodelay(true).unwrap();
        let listener = socket.listen(backlog).unwrap();

        listener
    };

    loop {
        let (mut stream, addr) = listener.accept().await.unwrap();
        stream.set_nodelay(true).unwrap();

        println!("new client: {}", addr);

        tokio::spawn(async move {
            let (reader, mut writer) = stream.split();

            let mut reader = BufReader::new(reader);

            let mut line = String::new();

            // "CONNECT hangj.cnblogs.com:443 HTTP/1.1\r\n"
            // "GET http://hangj.cnblogs.com/ HTTP/1.1\r\n"
            reader.read_line(&mut line).await.unwrap();
            println!("line: {line:?}");

            let vec = line.split(char::is_whitespace).filter(|s|!s.is_empty()).collect::<Vec<_>>();
            println!("vec: {vec:?}");

            assert_eq!(vec.len(), 3);

            let method = vec[0];
            let uri = vec[1];
            let version = vec[2];


            // find host, port and path
            let (host, port, path) = if uri.starts_with("http://") {
                let uri = &uri[7..];
                let idx = uri.find('/').unwrap_or(uri.len());
                let mut host = &uri[..idx];
                let path = &uri[idx..];
                let port = if let Some(idx) = host.find(':') {
                    let h = host;
                    host = &h[..idx];
                    h[idx + 1..].parse::<u16>().unwrap()
                } else {
                    80
                };
                (host, port, path)
            } else if uri.starts_with("https://") {
                let uri = &uri[8..];
                let idx = uri.find('/').unwrap_or(uri.len());
                let mut host = &uri[..idx];
                let path = &uri[idx..];
                let port = if let Some(idx) = host.find(':') {
                    let h = host;
                    host = &h[..idx];
                    h[idx + 1..].parse::<u16>().unwrap()
                } else {
                    443
                };
                (host, port, path)
            } else {
                match uri.find(':') {
                    Some(idx) => {
                        let host = &uri[..idx];
                        let port = uri[idx + 1..].parse::<u16>().unwrap();
                        let path = "";
                        (host, port, path)
                    }
                    None => {
                        eprintln!("invalid uri");
                        return;
                    }
                }
            };

            println!("host: {host}, port: {port}, path: {path}");

            let mut remote_stream = TcpStream::connect((host, port)).await.unwrap();
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
