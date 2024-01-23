use std::io::Write;

use clap::Parser;
use reqwest;
use select::{
    document::Document,
    predicate::{Attr, Name, Predicate},
};

#[derive(Debug)]
pub struct Link {
    pub base: String,
    pub html_name: String,
}

impl Link {
    pub fn new(html_name: String) -> Self {
        // 87537.html
        Link {
            base: "https://m.qqaiqin.com/article/".to_string(),
            html_name: html_name,
        }
    }
}

/// 从 m.qqaiqin.com 网站下载网课的答案。
#[derive(Parser, Debug)]
#[command(author = "Theo Cheng", version = "0.1.0", about, long_about = None)]
struct Args {
    #[arg(short, long)]
    /// such as: https://m.qqaiqin.com/article/87537.html -> 87537.html  就是地址栏中最后一个斜杠之后的部分
    id: String,

    #[arg(short, long, default_value_t = String::from("result.txt"))]
    /// 爬取的文本输出到什么地方
    output: String,
}

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let args = Args::parse();

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.36")
        .build()?;

    let result_text = get_page_source(&Link::new(args.id), &client).await?;
    let doc = Document::from(result_text.as_ref());
    if let Some(result) = extract_content(&doc).await {
        // println!("{}", result);
        append_to_file(&result, &args.output)
            .await
            .unwrap_or_else(|err| {
                println!("Error: {}", err);
                std::process::exit(1);
            });
    }
    if let Some(links) = get_other_urls(&doc).await {
        for link in links.iter() {
            let result_text = get_page_source(link, &client).await?;
            let doc = Document::from(result_text.as_ref());
            if let Some(result) = extract_content(&doc).await {
                // println!("{}", result);
                append_to_file(&result, &args.output)
                    .await
                    .unwrap_or_else(|err| {
                        println!("Error: {}", err);
                        std::process::exit(1);
                    });
            }
        }
    }
    Ok(())
}

async fn append_to_file(s: &str, file_path: &str) -> Result<(), std::io::Error> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)?;

    file.write_all(s.as_bytes())?;
    Ok(())
}

async fn get_page_source(link: &Link, client: &reqwest::Client) -> Result<String, reqwest::Error> {
    let url = format!("{}{}", link.base, link.html_name);
    let raw_req_body = client.get(url).send().await?.bytes().await?;

    let (result_text, _, _) = encoding_rs::GBK.decode(&raw_req_body);
    Ok(result_text.into_owned())
}

async fn extract_content(doc: &Document) -> Option<String> {
    let mut result = String::new();
    let p_ls: Vec<select::node::Node<'_>> = doc.find(Name("p")).collect();
    for (index, node) in p_ls.iter().enumerate() {
        if index < p_ls.len() - 2 {
            result.push_str(&node.text());
            result.push_str("\n");
        }
    }
    Some(result)
}

async fn get_other_urls(doc: &Document) -> Option<Vec<Link>> {
    let mut urls: Vec<Link> = Vec::new();

    for ls in doc.find(Name("div").and(Attr("class", "m_pages"))) {
        for li in ls.find(Name("li")) {
            let href = match li.find(Name("a")).next().unwrap().attr("href") {
                Some(href) => href,
                None => continue,
            };
            if href.ends_with(".html") {
                urls.push(Link::new(href.to_string()));
            }
        }
    }

    urls.pop();

    Some(urls)
}
