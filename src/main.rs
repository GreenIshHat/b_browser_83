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

// Extract top N largest text blocks, skipping scripts/styles and deduping.
fn extract_largest_text_blocks(html: &str, count: usize) -> Vec<String> {
    let doc = Html::parse_document(html);
    let selectors = [
        Selector::parse("article").unwrap(),
        Selector::parse("section").unwrap(),
        Selector::parse("div").unwrap(),
        Selector::parse("p").unwrap(),
    ];
    let mut blocks = vec![];
    for sel in selectors.iter() {
        for el in doc.select(sel) {
            // Skip if inside <script> or <style>
            let mut parent = el.parent();
            let mut skip = false;
            while let Some(p) = parent {
                if let Some(elem) = p.value().as_element() {
                    let tag = elem.name();
                    if tag == "script" || tag == "style" {
                        skip = true;
                        break;
                    }
                }
                parent = p.parent();
            }
            if skip { continue; }
            let t = el.text().collect::<String>().trim().to_string();
            if t.len() > 60 {
                blocks.push(t);
            }
        }
    }
    blocks.sort();
    blocks.dedup();
    blocks.sort_by(|a, b| b.len().cmp(&a.len()));
    blocks.into_iter().take(count).collect()
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

        println!("--- HTML PAGE ---");
        let doc = Html::parse_document(&body);
        let sel_a = Selector::parse("a").unwrap();

        // Show top 3 largest blocks, each max 1000 chars, separated by blank lines
        let top_blocks = extract_largest_text_blocks(&body, 3);
        if top_blocks.is_empty() {
            println!("(No significant text blocks found.)");
        } else {
            for (i, block) in top_blocks.iter().enumerate() {
                let out = if block.len() > 1000 {
                    format!("{}... [truncated]", &block[..1000])
                } else {
                    block.clone()
                };
                println!("--- Main Text Block {} ---\n{}\n", i + 1, out);
            }
        }

        // List all links, paginated, with range and back/next options
        let links: Vec<String> = doc.select(&sel_a)
            .filter_map(|el| el.value().attr("href").map(|l| l.to_string()))
            .collect();

        if links.is_empty() {
            println!("[*] No links found. Exiting.");
            break;
        }

		let page_size = 10;
		let mut page = 0;
		let total = links.len();
		let mut expanded = false; // tracks if main text blocks are expanded

		loop {
			// Show main text blocks, truncated or not
			if top_blocks.is_empty() {
				println!("(No significant text blocks found.)");
			} else {
				for (i, block) in top_blocks.iter().enumerate() {
				    let out = if expanded || block.len() <= 1000 {
				        block.clone()
				    } else {
				        format!("{}... [truncated]", &block[..1000])
				    };
				    println!("--- Main Text Block {} ---\n{}\n", i + 1, out);
				}
			}

			// Show link range and navigation
			let start = page * page_size;
			let end = usize::min(start + page_size, total);
			println!("--- LINKS ON PAGE ({}–{} of {}) ---", start + 1, end, total);
			for (i, url) in links[start..end].iter().enumerate() {
				let abs = resolve_url(&current_url, url);
				println!("[{}] {}", i + 1, abs);
			}
			if end < total {
				println!("[n] Next page");
			}
			if page > 0 {
				println!("[b] Back");
			}
			if !expanded && !top_blocks.is_empty() {
				println!("[e] Expand article text");
			}
			println!("[q] Quit");
			let input = prompt("Pick a link number, 'e' to expand, 'n' for next, 'b' for back, or 'q' to quit: ");
			if input == "q" {
				print!("\x07");
				use std::io::Write;
				std::io::stdout().flush().unwrap();
				return;
			}
			if input == "e" && !expanded {
				expanded = true;
				continue;
			}
			if input == "n" && end < total {
				page += 1;
				continue;
			}
			if input == "b" && page > 0 {
				page -= 1;
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
    // Extra safety: bell on program end.
    print!("\x07");
    use std::io::Write;
    std::io::stdout().flush().unwrap();
}

