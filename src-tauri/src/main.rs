#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::{
    marker::Unpin,
    ops::RangeInclusive,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::bail;
use futures::stream::StreamExt;
use reqwest::Client;
use shared::{Credentials, Message, YearMonth};
use tauri::{LogicalSize, Manager, Window};
use tokio::{
    fs::File,
    io::{AsyncWrite, AsyncWriteExt, BufWriter},
};

#[tauri::command]
fn default_save_location() -> Option<PathBuf> {
    dirs::download_dir().or_else(|| std::env::current_dir().ok())
}

#[tauri::command]
async fn start(
    window: Window,
    creds: Credentials,
    range: RangeInclusive<YearMonth>,
    save_location: PathBuf,
) -> Result<(), String> {
    start_body(&window, creds, range, save_location)
        .await
        .map_err(|err| err.to_string())
}

async fn start_body(
    window: &Window,
    creds: Credentials,
    range: RangeInclusive<YearMonth>,
    save_location: PathBuf,
) -> anyhow::Result<()> {
    let client = Arc::new(reqwest::Client::builder().cookie_store(true).build()?);
    login(window, &client, creds).await?;
    download_concurrently(window, client, &range, &save_location).await?;
    Ok(())
}

async fn login(window: &Window, client: &Client, creds: Credentials) -> anyhow::Result<()> {
    window.emit_all("output", Message::message("ログイン中...".into()))?;
    let form = reqwest::multipart::Form::new()
        .text("loginId", creds.email.clone())
        .text("loginPass", creds.password.clone());
    let body = client
        .post("https://app.ixsie.jp/signin")
        .multipart(form)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    if !body.contains("ログアウト") {
        bail!("ログインに失敗しました。ログイン情報を確認してください。");
    }
    window.emit_all("output", Message::message("ログイン成功".into()))?;
    Ok(())
}

async fn download_concurrently(
    window: &Window,
    client: Arc<Client>,
    range: &RangeInclusive<YearMonth>,
    save_location: &Path,
) -> anyhow::Result<()> {
    let months = tokio_stream::iter(YearMonth::iter_range(range));
    let mut stream = months
        .map(move |month| {
            let url = generate_url(month);
            let client = Arc::clone(&client);
            async move {
                let mut output = output_file(save_location, month).await?;
                download(&client, &mut output, &url).await?;
                output.flush().await?;
                anyhow::Ok(month)
            }
        })
        .buffer_unordered(4);
    while let Some(res) = stream.next().await {
        let message = res.map_or_else(Message::from, Message::from);
        window.emit_all("output", message)?;
    }
    window.emit_all("output", Message::message("完了".into()))?;
    Ok(())
}

fn generate_url(month: YearMonth) -> String {
    format!(
        "https://app.ixsie.jp/user/contact/pdf?contactYear={}&contactMonth={}",
        month.year,
        month.month.number_from_month()
    )
}

async fn output_file(save_location: &Path, month: YearMonth) -> anyhow::Result<BufWriter<File>> {
    let file = tokio::fs::File::create(save_location.join(format!("{month}.pdf"))).await?;
    Ok(BufWriter::new(file))
}

async fn download(
    client: &Client,
    mut writer: impl AsyncWrite + Unpin,
    url: &str,
) -> anyhow::Result<()> {
    let mut response = client.get(url).send().await?.error_for_status()?;
    while let Some(chunk) = response.chunk().await? {
        writer.write_all(&chunk).await?;
    }
    writer.flush().await?;
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            if let Some(window) = app.get_window("main") {
                window.set_min_size(Some(LogicalSize::new(300.0, 800.0)))?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![default_save_location, start])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
