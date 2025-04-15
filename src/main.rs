use tokio::{
    io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut iter = std::env::args();
    let exe = iter.next().unwrap();
    let Some(addr) = iter.next() else {
        eprintln!("Usage: {exe} <ip:port>\r\nExample: {exe} 127.0.0.1:1081");
        std::process::exit(1);
    };
    let listener = TcpListener::bind(addr).await?;

    loop {
        let (mut stream, _addr) = listener.accept().await?;
        stream.set_nodelay(true)?;

        tokio::spawn(async move {
            let (reader, mut writer) = stream.split();

            let mut reader = BufReader::new(reader);

            let mut line = String::with_capacity(128);

            // "CONNECT hangj.cnblogs.com:443 HTTP/1.1\r\n"
            // "GET http://hangj.cnblogs.com/ HTTP/1.1\r\n"
            reader.read_line(&mut line).await?;
            let mut it = line.trim().split(char::is_whitespace).filter(|s|!s.is_empty());

            let method = it.next().unwrap();
            let uri = it.next().unwrap();
            let version = it.next().unwrap();

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
                    // eprintln!("Invalid uri: {uri}");
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Invalid uri: {uri}"),
                    ));
                };
                (host, port, path)
            };

            println!("method: {method:?}, host: {host:?}, port: {port}, path: {path:?}");

            let mut remote_stream = TcpStream::connect((host, port)).await?;
            let _ = remote_stream.set_nodelay(true);

            if method.eq_ignore_ascii_case("CONNECT") {
                let mut header = String::with_capacity(128);
                loop {
                    header.clear();
                    reader.read_line(&mut header).await?;

                    // "Proxy-Authorization: Basic Ym9iOmFsaWNl\r\n"
                    println!("{header:?}");

                    if header == "\r\n" {
                        break;
                    }
                }
                writer.write_all(version.as_bytes()).await?;
                writer.write_all(b" 200 Connection Established\r\n\r\n").await?;
                remote_stream.write_all(reader.buffer()).await?;
            } else {
                remote_stream.write_all(format!("{method} {path} {version}\r\n").as_bytes()).await?;

                let mut header = String::with_capacity(128);
                loop {
                    header.clear();
                    reader.read_line(&mut header).await?;

                    // "Proxy-Authorization: Basic Ym9iOmFsaWNl\r\n"
                    println!("{header:?}");
                    if !header.to_lowercase().starts_with("proxy-") {
                        remote_stream.write_all(header.as_bytes()).await?;
                    }

                    if header == "\r\n" {
                        break;
                    }
                }

                remote_stream.write_all(reader.buffer()).await?;
            }

            let _ = tokio::io::copy_bidirectional(&mut stream, &mut remote_stream).await?;

            Ok(())
        });
    }
}
