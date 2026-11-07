//! Nexus refuses GraphQL queries whose complexity is over 10000. The owner's device got "Query has complexity of
//! 18440, which exceeds max complexity of 10000" for the mod page in 0.5, when its details query also asked for 100
//! requirements. This estimates complexity the way that number works out (each field 1; a list field multiplies what
//! it holds by its `count:`, or by 20 when it has none) and keeps every query Turbine sends well under the limit.

use turbine_engine::json;
use turbine_engine::nexus::{self, SearchSort};

/// List fields: they multiply their selection (by `count:`, or 20).
const LISTS: &[&str] = &["legacyModsByDomain", "mods", "modFiles", "nexusRequirements", "dlcRequirements"];

fn estimate(query: &str) -> u64 {
    // Tokens: names, braces, and the argument lists (kept whole, for count).
    let mut toks: Vec<String> = Vec::new();
    let mut chars = query.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '(' {
            let mut depth = 1;
            let mut arg = String::new();
            for d in chars.by_ref() {
                if d == '(' {
                    depth += 1;
                } else if d == ')' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                arg.push(d);
            }
            toks.push(format!("({arg}"));
        } else if c == '{' || c == '}' {
            toks.push(c.to_string());
        } else if c.is_alphanumeric() || c == '_' || c == '$' {
            let mut w = c.to_string();
            while let Some(&d) = chars.peek() {
                if d.is_alphanumeric() || d == '_' {
                    w.push(d);
                    chars.next();
                } else {
                    break;
                }
            }
            toks.push(w);
        }
    }
    fn selection(t: &[String], i: &mut usize) -> u64 {
        let mut cost = 0;
        while *i < t.len() && t[*i] != "}" {
            let name = t[*i].clone();
            *i += 1;
            let mut count: Option<u64> = None;
            if *i < t.len() && t[*i].starts_with('(') {
                let a = &t[*i];
                if a.contains("count: $count") {
                    count = Some(100); // a variable: the most Turbine ever asks for
                } else if let Some(p) = a.find("count:") {
                    count = a[p + 6..].trim().split(|c: char| !c.is_ascii_digit()).next().and_then(|n| n.parse().ok());
                }
                *i += 1;
            }
            if *i < t.len() && t[*i] == "{" {
                *i += 1;
                let inner = selection(t, i);
                *i += 1; // "}"
                let mult = count.unwrap_or(if LISTS.contains(&name.as_str()) { 20 } else { 1 });
                cost += 1 + mult * inner;
            } else {
                cost += 1;
            }
        }
        cost
    }
    // Skip "query Name" up to the first "{".
    let start = toks.iter().position(|t| t == "{").unwrap() + 1;
    let mut i = start;
    selection(&toks, &mut i)
}

fn query_of(body: &str) -> String {
    json::parse(body).unwrap().str_at("query").unwrap().to_string()
}

#[test]
fn every_query_stays_well_under_the_limit() {
    // The 0.5 details query, as sent: the estimate lands near what Nexus reported, so the rule is a fair one.
    let old = "query TurbineMod($ids: [CompositeDomainWithIdInput!]!) { legacyModsByDomain(ids: $ids) { nodes { modId name \
summary version author pictureUrl thumbnailUrl endorsements downloads adultContent status updatedAt game { id domainName } \
modRequirements { nexusRequirements(count: 100) { totalCount nodes { id modId modName gameId url notes externalRequirement } } \
dlcRequirements { notes gameExpansion { id name } } } } } }";
    let old_cost = estimate(old);
    assert!((15_000..25_000).contains(&old_cost), "{old_cost}");

    let bodies = [
        ("details", nexus::details_body("witcher3", 5)),
        ("requirements", nexus::requirements_body("witcher3", 5)),
        ("files", nexus::files_body(952, 5)),
        ("search", nexus::search_body("witcher3", "torch", SearchSort::Trending, 0, 30, false)),
        ("search 100", nexus::search_body("witcher3", "", SearchSort::Endorsements, 0, 100, false)),
        ("updates", nexus::by_id_body("witcher3", &(1..=50).collect::<Vec<u64>>())),
    ];
    for (what, body) in bodies {
        let q = query_of(&body);
        let cost = estimate(&q);
        assert!(cost <= 5_000, "{what}: {cost} (half the limit at most)\n{q}");
        // No list asks for more than the page needs.
        for part in q.split("count: ").skip(1) {
            let Ok(n) = part.split(|c: char| !c.is_ascii_digit()).next().unwrap().parse::<u64>() else { continue };
            assert!(n <= 100, "{what}: count {n}");
        }
    }
    // The details query no longer carries the requirements; they are their own request with a short list.
    assert!(!query_of(&nexus::details_body("witcher3", 5)).contains("modRequirements"));
    assert!(query_of(&nexus::requirements_body("witcher3", 5)).contains(&format!("count: {}", nexus::MAX_REQUIREMENTS)));
}

/// With TURBINE_QUERY_OUT set, writes the exact request bodies to that folder, so the build's live Nexus check can
/// send them as they are.
#[test]
fn write_bodies_for_the_live_check() {
    let Ok(dir) = std::env::var("TURBINE_QUERY_OUT") else { return };
    std::fs::create_dir_all(&dir).unwrap();
    let write = |name: &str, body: String| std::fs::write(format!("{dir}/{name}.json"), body).unwrap();
    write("search-witcher3", nexus::search_body("witcher3", "", SearchSort::Trending, 0, 30, false));
    write("search-newest-witcher3", nexus::search_body("witcher3", "", SearchSort::Newest, 0, 30, false));
    write("search-text-skyrimspecialedition", nexus::search_body("skyrimspecialedition", "lantern", SearchSort::Endorsements, 0, 30, false));
    write("search-category-skyrimspecialedition", nexus::search_body_in("skyrimspecialedition", "", "Weapons", SearchSort::Endorsements, 0, 30, false, 0));
    write("details-witcher3-1021", nexus::details_body("witcher3", 1021));
    write("requirements-skyrimspecialedition-12604", nexus::requirements_body("skyrimspecialedition", 12604));
    write("files-witcher3-1021", nexus::files_body(952, 1021));
    write("updates-skyrimspecialedition", nexus::by_id_body("skyrimspecialedition", &[12604, 32444, 266]));
    write("details-kingdomcomedeliverance2-14", nexus::details_body("kingdomcomedeliverance2", 14));
}
