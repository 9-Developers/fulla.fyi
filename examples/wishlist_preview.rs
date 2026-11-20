//! Render a standalone wishlist preview using the same card template and built CSS as the site.

use askama::Template;

struct Item<'a> {
    name: &'a str,
    retailer_url: &'a str,
    priority: &'a str,
    desired_quantity: u32,
}

#[derive(Template)]
#[template(path = "wishlist_card.html")]
struct WishlistCard<'a> {
    title: &'a str,
    items: &'a [Item<'a>],
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let palette = std::str::from_utf8(nine_dev_site_lib::assets::theme::PALETTE.bytes)?;
    let css = include_str!("../public/main.css");
    println!(
        r#"<!doctype html><html lang="en" data-theme="latte"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1"><title>Wishlist preview</title>
<style>{palette}</style><style>{css}</style></head><body><main><h1>Wishlists</h1>"#
    );
    let items = [Item {
        name: "High-capacity backup drive — DesktopStorageExpansionKitWithAnUnusuallyLongModelNumber123456789",
        retailer_url: "https://example.com/drives/backup",
        priority: "High",
        desired_quantity: 4,
    }];
    for card in [
        WishlistCard {
            title: "Home office & family gift ideas",
            items: &items,
        },
        WishlistCard {
            title: "Next birthday",
            items: &[],
        },
    ] {
        println!("{}", card.render()?);
    }
    println!("</main></body></html>");
    Ok(())
}
