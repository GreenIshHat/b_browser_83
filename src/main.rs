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

fn normalize_url(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("https://{}", url)
    }
}

fn resolve_url(base: &str, link: &str) -> String {
    if link.starts_with("http") {
        link.to_string()
    } else {
        url::Url::parse(base)
            .and_then(|b| b.join(link))
            .map(|u| u.to_string())
            .unwrap_or_else(|_| link.to_string())
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
            println!("Invalid selection, try again.");
            continue;
        }

        // HTML page flow
        println!("--- HTML PAGE ---");
        let doc = Html::parse_document(&body);
        let sel_p = Selector::parse("p").unwrap();
        let sel_a = Selector::parse("a").unwrap();

        // Find the biggest <p> or text block
        let mut biggest = String::new();
        for el in doc.select(&sel_p) {
            let t = el.text().collect::<String>().trim().to_string();
            if t.len() > biggest.len() {
                biggest = t;
            }
        }
        if biggest.is_empty() {
            println!("(No significant <p> text found.)");
        } else {
            println!("--- Main Text ---\n{}\n", biggest);
        }

        // List all links, paginated
        let links: Vec<String> = doc.select(&sel_a)
            .filter_map(|el| el.value().attr("href").map(|l| l.to_string()))
            .collect();

        if links.is_empty() {
            println!("[*] No links found. Exiting.");
            break;
        }

        let page_size = 10;
        let mut page = 0;
        loop {
            let start = page * page_size;
            let end = usize::min(start + page_size, links.len());
            println!("--- LINKS ON PAGE ---");
            for (i, url) in links[start..end].iter().enumerate() {
                let abs = resolve_url(&current_url, url);
                println!("[{}] {}", i + 1, abs);
            }
            if end < links.len() {
                println!("[n] Next page");
            }
            println!("[q] Quit");
            let input = prompt("Pick a link number, 'n' for next, or 'q' to quit: ");
            if input == "q" { return; }
            if input == "n" && end < links.len() {
                page += 1;
                continue;
            }
            if let Ok(n) = input.parse::<usize>() {
                if n > 0 && n <= end - start {
                    let abs = resolve_url(&current_url, &links[start + n - 1]);
                    current_url = abs;
                    break;
                }
            }
            println!("Invalid input, try again.");
        }
    }
}

