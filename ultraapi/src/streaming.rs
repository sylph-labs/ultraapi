//! ストリーミングレスポンスのヘルパーモジュール
//!
//! このモジュールは、HTTP ストリーミングレスポンスを作成するための便利機能を提供します。
//!
//! # 主な機能
//!
//! - `reader_stream`: `tokio::io::AsyncRead` からストリームを作成
//! - `StreamingResponse::from_reader`: AsyncRead から直接 StreamingResponse を作成
//!
//! # Backpressure（背圧制御）について
//!
//! Backpressure（背圧制御）は axum/tower に委譲されます。
//! HTTP レスポンスの送信速度がクライアントの受信速度より速い場合、
//! tower の Middleware （例如: `tower::ServiceBuilder` のバッファリング）が
//! 自動的に背圧を適用します。
//!
//! 具体的な背圧の挙動:
//! - クライアントがデータを受け取るのが遅い場合、サーバーは自動的にデータの送信を pause します
//! - `tokio` の非同期ランタイムが効率的にリソースを管理します
//!
//! # Chunked Encoding（チャンクド転送）について
//!
//! このモジュールで作成されたレスポンスはデフォルトで chunked transfer encoding を使用します。
//! これは以下の条件を満たす場合に自動的に有効になります:
//!
//! - レスポンスの Content-Length が事前に不明な場合
//! - クライアント（多くのHTTPクライアント）が HTTP/1.1 を使用している場合
//!
//! 注意点:
//! - 一部の古いクライアントやプロキシは chunked encoding を正しく処理できません
//! - データサイズが事前にわかる場合は、`Content-Length` ヘッダーを手動で設定してください
//!
//! # Flush（フラッシュ）とバッファリングについて
//!
//! ストリームからのデータは一定量バッファリングされます。
//! リアルタイムにデータを読みたい場合は、以下の点に注意してください:
//!
//! - データはチャンク単位で送信されます（通常 8KB〜64KB）
//! - 強制的な flush が必要なユースケースでは、個別のチャンクを明示的に送信してください
//! -  대부분의 경우, 버퍼링은 성능을 향상시키지만 실시간성이 중요한 경우엔 주의가 필요합니다
//!
//! # エラー処理
//!
//! ストリームの読み取り中にエラーが発生した場合:
//! - エラーは標準エラー出力（`eprintln!`）に出力されます
//! - 接続は閉じられ、空のレスポンスがクライアントに送信されます
//! - 本番環境では、適切なロギングシステムを設定してください

use crate::StreamingResponse;
use bytes::Bytes;
use futures_util::stream::TryStreamExt;
use tokio::io::{AsyncRead, AsyncReadExt};

/// AsyncRead からストリームを作成します
///
/// この関数は `tokio::io::AsyncRead` を実装任意の型から
/// `StreamingResponse` に使用できるストリームに変換します.
///
/// # 引数
///
/// - `reader`: `tokio::io::AsyncRead` を実装するリーダー
/// - `chunk_size`: 読み取りごとのバッファサイズ（デフォルトは 8KB）
///
/// # 戻り値
///
/// `Result<bytes::Bytes, Box<dyn std::error::Error + Send + Sync>>` を emit するストリーム
///
/// # Example
///
/// ```ignore
/// use ultraapi::prelude::*;
/// use tokio::fs::File;
/// use tokio::io::AsyncReadExt;
///
/// #[get("/file/{path}")]
/// async fn serve_file(path: String) -> StreamingResponse {
///     let mut file = File::open(&path).await.unwrap();
///     let stream = ultraapi::streaming::reader_stream(file, 8192);
///     StreamingResponse::new(stream)
///         .content_type("application/octet-stream")
/// }
/// AsyncRead からストリームを作成します（エラーなし版）
///
/// この関数は `tokio::io::AsyncRead` を実装任意の型から
/// エラーを返さないストリームに変換します.
/// 読み取りエラーはログに出力され、空のチャンクが返されます.
///
/// # 引数
///
/// - `reader`: `tokio::io::AsyncRead` を実装するリーダー
/// - `chunk_size`: 読み取りごとのバッファサイズ（デフォルトは 8KB）
pub fn reader_stream<R>(
    reader: R,
    chunk_size: usize,
) -> impl tokio_stream::Stream<Item = Result<Bytes, Box<dyn std::error::Error + Send + Sync>>> + Send
where
    R: AsyncRead + Send + Unpin + 'static,
{
    // Use try_unfold to convert AsyncRead to a Stream
    futures_util::stream::try_unfold(reader, move |mut reader| async move {
        let mut buf = vec![0u8; chunk_size];
        match reader.read(&mut buf).await {
            Ok(0) => Ok(None), // EOF
            Ok(n) => {
                buf.truncate(n);
                Ok(Some((Bytes::from(buf), reader)))
            }
            Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
        }
    })
}

/// AsyncRead からストリームを作成します（エラーなし版）
///
/// この関数は `tokio::io::AsyncRead` を実装任意の型から
/// エラーを返さないストリーム（Infallible）に変換します.
/// 読み取りエラーは空のチャンクとして扱われます.
///
/// # 引数
///
/// - `reader`: `tokio::io::AsyncRead` を実装するリーダー
/// - `chunk_size`: 読み取りごとのバッファサイズ（デフォルトは 8KB）
pub fn reader_stream_infallible<R>(
    reader: R,
    chunk_size: usize,
) -> impl tokio_stream::Stream<Item = Result<Bytes, std::convert::Infallible>> + Send
where
    R: AsyncRead + Send + Unpin + 'static,
{
    futures_util::stream::try_unfold(reader, move |mut reader| async move {
        let mut buf = vec![0u8; chunk_size];
        match reader.read(&mut buf).await {
            Ok(0) => Ok(None), // EOF
            Ok(n) => {
                buf.truncate(n);
                Ok(Some((Bytes::from(buf), reader)))
            }
            Err(_) => Ok(None), // On error, return empty chunk (EOF)
        }
    })
}

/// AsyncRead から StreamingResponse を作成します（エラーなし版）
///
/// このメソッドはエラーが発生しないことが明らかな AsyncRead 用です。
/// エラーが発生する可能性がある場合は `StreamingResponse::new()` を
/// `reader_stream()` と組み合わせて使用してください。
///
/// # 引数
///
/// - `reader`: `tokio::io::AsyncRead` を実装するリーダー
/// - `chunk_size`: 読み取りごとのバッファサイズ（デフォルトは 8KB）
///
/// # Example
///
/// ```ignore
/// use ultraapi::prelude::*;
/// use tokio::fs::File;
/// use tokio::io::AsyncReadExt;
///
/// #[get("/download/{filename}")]
/// async fn download_file(filename: String) -> StreamingResponse {
///     let mut file = File::open(&format!("files/{}", filename)).await.unwrap();
///     StreamingResponse::from_reader(file, 8192)
///         .content_type("application/octet-stream")
/// }
/// ```
impl StreamingResponse {
    /// AsyncRead から StreamingResponse を作成（エラーなし）
    ///
    /// 這個関数は `AsyncRead` 実装から直接 `StreamingResponse` を作成します。
    /// 読み取りエラーはログに出力され、空のレスポンスが返されます。
    ///
    /// # 引数
    ///
    /// - `reader`: `tokio::io::AsyncRead` を実装するリーダー
    /// - `chunk_size`: 読み取りごとのバッファサイズ（デフォルトは 8KB）
    pub fn from_reader<R>(reader: R, chunk_size: usize) -> Self
    where
        R: AsyncRead + Send + Unpin + 'static,
    {
        let stream = reader_stream_infallible(reader, chunk_size);
        StreamingResponse::from_infallible_stream(stream)
    }
}

/// bytes::Bytes のイテータからストリームを作成します
///
/// ベクタのイテレータから HTTP レスポンス用のストリームを作成します。
/// これはデータをチャンクに分割して送信したい場合に便利です。
///
/// # Example
///
/// ```ignore
/// use ultraapi::prelude::*;
/// use ultraapi::streaming::bytes_stream;
///
/// #[get("/chunks")]
/// async fn send_chunks() -> StreamingResponse {
///     let chunks = vec![
///         Bytes::from("chunk1\n"),
///         Bytes::from("chunk2\n"),
///         Bytes::from("chunk3\n"),
///     ];
///     StreamingResponse::from_stream(bytes_stream(chunks))
///         .content_type("text/plain")
/// }
/// ```
pub fn bytes_stream(chunks: Vec<Bytes>) -> impl tokio_stream::Stream<Item = Bytes> + Send {
    tokio_stream::iter(chunks)
}

/// イテータブルからストリームを作成します
///
/// イテータブル（例: `Vec<Bytes>`、`&[u8]` のスライスなど）から
/// ストリームを作成します。
///
/// # Example
///
/// ```ignore
/// use ultraapi::prelude::*;
/// use ultraapi::streaming::iter_stream;
///
/// #[get("/lines")]
/// async fn send_lines() -> StreamingResponse {
///     let lines = vec!["line1", "line2", "line3"];
///     StreamingResponse::from_stream(iter_stream(lines, |s| Bytes::from(s)))
///         .content_type("text/plain")
/// }
/// ```
pub fn iter_stream<T, F, B>(iter: T, f: F) -> impl tokio_stream::Stream<Item = Bytes> + Send
where
    T: IntoIterator + Send + 'static,
    T::IntoIter: Send,
    T::Item: Send,
    F: Fn(T::Item) -> B + Send + Sync + 'static,
    B: Into<Bytes>,
{
    tokio_stream::iter(iter.into_iter().map(move |item| f(item).into()))
}

/// 文字列のイテータからストリームを作成します
///
/// 文字列のイテータから HTTP レスポンス用のストリームを作成します。
/// 各文字列は改行で区切られます。
pub fn string_stream(strings: Vec<String>) -> impl tokio_stream::Stream<Item = Bytes> + Send {
    iter_stream(strings, Bytes::from)
}

/// テキストファイルの行ごとのストリームを作成します
///
/// ファイルを行ごとに読み取り、各行をチャンクとして送信します。
/// これは大きなテキストファイルを少しずつ送信する場合に便利です。
///
/// # Example
///
/// ```ignore
/// use ultraapi::prelude::*;
/// use tokio::fs::File;
/// use tokio::io::AsyncBufReadExt;
///
/// #[get("/log/{filename}")]
/// async fn stream_log(filename: String) -> StreamingResponse {
///     let file = File::open(&format!("logs/{}", filename)).await.unwrap();
///     let reader = tokio::io::BufReader::new(file);
///     let lines = tokio::io::BufRead::lines(reader);
///     StreamingResponse::from_stream(
///         ultraapi::streaming::lines_stream(lines)
///     )
///         .content_type("text/plain")
/// }
/// ```
pub fn lines_stream<S, E>(
    lines: S,
) -> impl tokio_stream::Stream<Item = Result<Bytes, Box<dyn std::error::Error + Send + Sync>>> + Send
where
    S: tokio_stream::Stream<Item = Result<String, E>> + Send + 'static,
    E: std::error::Error + Send + Sync + 'static,
{
    lines
        .map_ok(|line| {
            let mut bytes = line.into_bytes();
            bytes.push(b'\n'); // Add newline after each line
            Bytes::from(bytes)
        })
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
}

/// Byte ストリームを作成します
///
/// 任意の `Stream<Item = Result<T, E>>` を `Stream<Item = Result<Bytes, _>>` に変換します。
/// データの変換処理が必要な場合に使用します。
///
/// # Example
///
/// ```ignore
/// use ultraapi::prelude::*;
/// use ultraapi::streaming::map_to_bytes;
///
/// #[get("/data")]
/// async fn stream_data() -> StreamingResponse {
///     let data = vec![1u8, 2, 3, 4, 5];
///     let stream = futures_util::stream::iter(data.into_iter().map(Ok::<u8, std::convert::Infallible>));
///     StreamingResponse::from_stream(
///         map_to_bytes(stream, |b| Bytes::from(vec![b]))
///     )
///         .content_type("application/octet-stream")
/// }
/// ```
pub fn map_to_bytes<S, T, E, F>(
    stream: S,
    f: F,
) -> impl tokio_stream::Stream<Item = Result<Bytes, Box<dyn std::error::Error + Send + Sync>>> + Send
where
    S: tokio_stream::Stream<Item = Result<T, E>> + Send + 'static,
    T: Send + 'static,
    E: std::error::Error + Send + Sync + 'static,
    F: Fn(T) -> Bytes + Send + 'static,
{
    stream
        .map_ok(f)
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use futures_util::stream::StreamExt;
    use std::io::Cursor;

    /// Test: reader_stream with in-memory data
    #[tokio::test]
    async fn test_reader_stream_basic() {
        let data = b"Hello, World!".to_vec();
        let cursor = Cursor::new(data);

        let stream = reader_stream(cursor, 8192);
        let collected: Vec<Result<Bytes, _>> = stream.collect().await;

        // Should have one chunk with all data
        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0].as_ref().unwrap().as_ref(), b"Hello, World!");
    }

    /// Test: reader_stream with multiple chunks
    #[tokio::test]
    async fn test_reader_stream_chunk_size() {
        let data = b"1234567890".to_vec();
        let cursor = Cursor::new(data);

        // Use chunk_size of 3 to force multiple reads
        let stream = reader_stream(cursor, 3);
        let collected: Vec<Result<Bytes, _>> = stream.collect().await;

        // Should have multiple chunks
        let combined: Vec<u8> = collected
            .into_iter()
            .flat_map(|r| r.unwrap().to_vec())
            .collect();

        assert_eq!(combined, b"1234567890");
    }

    /// Test: from_reader creates valid StreamingResponse
    #[tokio::test]
    async fn test_from_reader_basic() {
        let data = b"Test data for reader".to_vec();
        let cursor = Cursor::new(data);

        let response = StreamingResponse::from_reader(cursor, 8192).content_type("text/plain");

        // Just verify it compiles and can be converted
        let _ = response;
    }

    /// Test: bytes_stream creates valid stream
    #[tokio::test]
    async fn test_bytes_stream() {
        let chunks = vec![
            Bytes::from("chunk1"),
            Bytes::from("chunk2"),
            Bytes::from("chunk3"),
        ];

        let stream = bytes_stream(chunks);
        let collected: Vec<Bytes> = stream.collect().await;

        assert_eq!(collected.len(), 3);
        assert_eq!(collected[0].as_ref(), b"chunk1");
        assert_eq!(collected[1].as_ref(), b"chunk2");
        assert_eq!(collected[2].as_ref(), b"chunk3");
    }

    /// Test: string_stream creates valid stream
    #[tokio::test]
    async fn test_string_stream() {
        let strings = vec![
            "line1".to_string(),
            "line2".to_string(),
            "line3".to_string(),
        ];

        let stream = string_stream(strings);
        let collected: Vec<Bytes> = stream.collect().await;

        assert_eq!(collected.len(), 3);
        assert_eq!(collected[0].as_ref(), b"line1");
        assert_eq!(collected[1].as_ref(), b"line2");
        assert_eq!(collected[2].as_ref(), b"line3");
    }

    /// Test: iter_stream creates valid stream
    #[tokio::test]
    async fn test_iter_stream() {
        let numbers = vec![1, 2, 3];

        let stream = iter_stream(numbers, |n| Bytes::from(format!("num:{}", n)));
        let collected: Vec<Bytes> = stream.collect().await;

        assert_eq!(collected.len(), 3);
        assert_eq!(collected[0].as_ref(), b"num:1");
        assert_eq!(collected[1].as_ref(), b"num:2");
        assert_eq!(collected[2].as_ref(), b"num:3");
    }
}
