use rayon::prelude::*;
use reqwest::blocking::get;
use feed_rs::parser;
use scraper::{Html, Selector};

fn prompt(msg: &str) -> String {
    use std::io::{self, Write};
    print!("{}", msg);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn fetch_url(url: &str) -> Result<String, reqwest::Error> {
    Ok(get(url)?.text()?)
}

fn try_parse_feed(xml: &str) -> Option<Vec<(String, String)>> {
    match parser::parse(xml.as_bytes()) {
        Ok(feed) if !feed.entries.is_empty() => Some(
            feed.entries
                .iter()
                .map(|e| {
                    let title = e.title.as_ref().map_or("Untitled", |t| t.content.as_str()).to_string();
                    let link = e.links.get(0).map_or("", |l| l.href.as_str()).to_string();
                    (title, link)
                })
                .collect()
        ),
        _ => None,
    }
}

fn extract_text(html: &str) -> (String, Vec<String>) {
    let doc = Html::parse_document(html);
    let sel_p = Selector::parse("p").unwrap();
    let sel_a = Selector::parse("a").unwrap();

    let text = doc.select(&sel_p)
        .map(|el| el.text().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");

    let links = doc.select(&sel_a)
        .filter_map(|el| el.value().attr("href").map(|l| l.to_string()))
        .collect();

    (text, links)
}

fn normalize_url(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("https://{}", url)
    }
}


fn main() {
    let mut current_url = normalize_url(&prompt("Enter feed or HTML URL: "));


    loop {
        println!("\n[+] Fetching: {}", current_url);
        let body = match fetch_url(&current_url) {
            Ok(b) => b,
            Err(e) => {
                println!("[!] Error fetching {}: {}", current_url, e);
                break;
            }
        };

        // Try to parse as feed first
        if let Some(items) = try_parse_feed(&body) {
            println!("--- FEED DETECTED ---");
            for (i, (title, link)) in items.iter().enumerate() {
                println!("[{}] {}", i + 1, title);
                println!("    {}", link);
            }
            let pick = prompt("Pick article number to open, or 'q' to quit: ");
            if pick == "q" { break; }
            if let Ok(n) = pick.parse::<usize>() {
                if n > 0 && n <= items.len() {
                    current_url = items[n - 1].1.clone();
                    continue;
                }
            }
            println!("Invalid selection, exiting.");
            break;
        }

// ... inside HTML handling block ...
println!("--- HTML PAGE ---");
let (text, links) = extract_text(&body);
println!("--- Content ---\n{}\n", text);

if links.is_empty() {
    println!("[*] No links found. Exiting.");
    break;
}

// Fetch content lengths in parallel
let mut links_with_len: Vec<_> = links.par_iter().map(|url| {
    let abs = if url.starts_with("http") {
        url.clone()
    } else {
        url::Url::parse(&current_url).and_then(|base| base.join(url)).map(|u| u.to_string()).unwrap_or(url.clone())
    };
    let len = fetch_url(&abs).ok().map(|body| body.len()).unwrap_or(0);
    (abs, len)
}).collect();

// Sort links by length, descending
links_with_len.sort_by(|a, b| b.1.cmp(&a.1));

// Paging
let page_size = 10;
let mut page = 0;
loop {
    let start = page * page_size;
    let end = usize::min(start + page_size, links_with_len.len());
    println!("--- LINKS ON PAGE (sorted by content length) ---");
    for (i, (url, len)) in links_with_len[start..end].iter().enumerate() {
        println!("[{}] {} (size: {})", i + 1, url, len);
    }
    if end < links_with_len.len() {
        println!("[n] Next page");
    }
    println!("[q] Quit");
    let input = prompt("Pick a link number, 'n' for next, or 'q' to quit: ");
    if input == "q" { break; }
    if input == "n" && end < links_with_len.len() {
        page += 1;
        continue;
    }
    if let Ok(n) = input.parse::<usize>() {
        if n > 0 && n <= end - start {
            current_url = links_with_len[start + n - 1].0.clone();
            break;
        }
    }
    println!("Invalid input, try again.");
}


