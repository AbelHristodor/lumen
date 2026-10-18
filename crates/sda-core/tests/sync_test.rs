use sda_core::sync::{sync_file, sync_hymns, SyncOutcome};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

/// Minimal HTTP/1.1 server answering HEAD and GET for one zip body. The ETag
/// is shared so a test can change it between requests.
fn serve(body: Vec<u8>, etag: Arc<Mutex<String>>) -> String {
    serve_shared(Arc::new(Mutex::new(body)), etag)
}

/// Like `serve` but the body is shared so a test can swap it between requests.
fn serve_shared(body: Arc<Mutex<Vec<u8>>>, etag: Arc<Mutex<String>>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/hymns.zip", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut stream = stream;
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let method = line.split_whitespace().next().unwrap_or("").to_string();
            loop {
                let mut h = String::new();
                if reader.read_line(&mut h).unwrap_or(0) == 0 || h == "\r\n" {
                    break;
                }
            }
            let etag = etag.lock().unwrap().clone();
            let body = body.lock().unwrap().clone();
            let head = format!(
                "HTTP/1.1 200 OK\r\nETag: {etag}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(head.as_bytes()).unwrap();
            if method == "GET" {
                stream.write_all(&body).unwrap();
            }
            let _ = stream.flush();
        }
    });
    url
}

fn zip_with(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut w = zip::ZipWriter::new(&mut buf);
        for (name, data) in files {
            w.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            w.write_all(data).unwrap();
        }
        w.finish().unwrap();
    }
    buf.into_inner()
}

fn read(path: &std::path::Path) -> Vec<u8> {
    let mut v = Vec::new();
    std::fs::File::open(path)
        .unwrap()
        .read_to_end(&mut v)
        .unwrap();
    v
}

#[test]
fn downloads_then_reports_unchanged_then_redownloads_on_new_etag() {
    let etag = Arc::new(Mutex::new("\"v1\"".to_string()));
    let url = serve(zip_with(&[("1-99/001.pptx", b"one")]), etag.clone());
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("hymns");
    let stamp = tmp.path().join("hymns.etag");

    assert_eq!(
        sync_hymns(&url, &dest, &stamp).unwrap(),
        SyncOutcome::Downloaded
    );
    assert_eq!(read(&dest.join("1-99/001.pptx")), b"one");
    assert_eq!(std::fs::read_to_string(&stamp).unwrap(), "\"v1\"");

    assert_eq!(
        sync_hymns(&url, &dest, &stamp).unwrap(),
        SyncOutcome::Unchanged
    );

    *etag.lock().unwrap() = "\"v2\"".to_string();
    assert_eq!(
        sync_hymns(&url, &dest, &stamp).unwrap(),
        SyncOutcome::Downloaded
    );
    assert_eq!(std::fs::read_to_string(&stamp).unwrap(), "\"v2\"");
}

#[test]
fn missing_library_folder_redownloads_even_with_matching_stamp() {
    let etag = Arc::new(Mutex::new("\"same\"".to_string()));
    let url = serve(zip_with(&[("a.pptx", b"x")]), etag);
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("hymns");
    let stamp = tmp.path().join("hymns.etag");
    std::fs::write(&stamp, "\"same\"").unwrap();

    assert_eq!(
        sync_hymns(&url, &dest, &stamp).unwrap(),
        SyncOutcome::Downloaded
    );
    assert!(dest.join("a.pptx").is_file());
    assert!(!tmp.path().join("hymns.part").exists());
    assert!(!tmp.path().join("hymns.tmp").exists());
}

#[test]
fn sync_file_downloads_then_unchanged() {
    let etag = Arc::new(Mutex::new("\"f1\"".to_string()));
    let body = b"bible bytes".to_vec();
    let url = serve(body.clone(), etag);
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("bibles").join("kjv.json");
    let stamp = tmp.path().join("bibles").join("kjv.etag");

    assert_eq!(
        sync_file(&url, &dest, &stamp).unwrap(),
        SyncOutcome::Downloaded
    );
    assert_eq!(read(&dest), body);
    assert_eq!(std::fs::read_to_string(&stamp).unwrap(), "\"f1\"");
    assert!(!dest.with_extension("part").exists());

    assert_eq!(
        sync_file(&url, &dest, &stamp).unwrap(),
        SyncOutcome::Unchanged
    );

    std::fs::remove_file(&dest).unwrap();
    assert_eq!(
        sync_file(&url, &dest, &stamp).unwrap(),
        SyncOutcome::Downloaded
    );
    assert_eq!(read(&dest), body);
}

#[test]
fn sync_hymns_failed_extract_leaves_no_stamp() {
    let etag = Arc::new(Mutex::new("\"bad\"".to_string()));
    let body = Arc::new(Mutex::new(b"this is not a zip".to_vec()));
    let url = serve_shared(body.clone(), etag);
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("hymns");
    let stamp = tmp.path().join("hymns.etag");

    assert!(sync_hymns(&url, &dest, &stamp).is_err());
    assert!(!stamp.exists());
    assert!(!dest.exists());
    assert!(!tmp.path().join("hymns.zip").exists());

    *body.lock().unwrap() = zip_with(&[("a.pptx", b"x")]);
    assert_eq!(
        sync_hymns(&url, &dest, &stamp).unwrap(),
        SyncOutcome::Downloaded
    );
    assert!(dest.join("a.pptx").is_file());
    assert_eq!(std::fs::read_to_string(&stamp).unwrap(), "\"bad\"");
}

#[test]
fn download_replaces_previous_library_contents() {
    let etag = Arc::new(Mutex::new("\"a\"".to_string()));
    let url = serve(zip_with(&[("new.pptx", b"n")]), etag);
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("hymns");
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::write(dest.join("old.pptx"), b"o").unwrap();

    sync_hymns(&url, &dest, &tmp.path().join("hymns.etag")).unwrap();
    assert!(!dest.join("old.pptx").exists());
    assert!(dest.join("new.pptx").is_file());
}
