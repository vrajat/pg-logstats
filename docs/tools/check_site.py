#!/usr/bin/env python3

from __future__ import annotations

from collections import Counter
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import urlparse


SITE_DIR = Path("site")
EXPECTED_ORIGIN = "https://pg-logstats.vrajat.com"


class HtmlHeadParser(HTMLParser):
    def __init__(self) -> None:
        super().__init__()
        self.in_title = False
        self.title = ""
        self.meta_description = None
        self.canonical = None
        self.links: list[str] = []
        self.json_ld_blocks = 0

    def handle_starttag(
        self, tag: str, attrs: list[tuple[str, str | None]]
    ) -> None:
        attrs_dict = dict(attrs)
        if tag == "title":
            self.in_title = True
        elif tag == "meta" and attrs_dict.get("name") == "description":
            self.meta_description = attrs_dict.get("content")
        elif tag == "link" and attrs_dict.get("rel") == "canonical":
            self.canonical = attrs_dict.get("href")
        elif tag == "script" and attrs_dict.get("type") == "application/ld+json":
            self.json_ld_blocks += 1

        for key in ("href", "src"):
            value = attrs_dict.get(key)
            if value:
                self.links.append(value)

    def handle_endtag(self, tag: str) -> None:
        if tag == "title":
            self.in_title = False

    def handle_data(self, data: str) -> None:
        if self.in_title:
            self.title += data


def resolve_relative_target(html_file: Path, link: str) -> Path:
    parsed = urlparse(link)
    path = parsed.path
    if link.startswith("/"):
        target = (SITE_DIR / path.lstrip("/")).resolve()
    else:
        target = (html_file.parent / path).resolve()
    if target.is_dir():
        target = target / "index.html"
    return target


def main() -> int:
    if not SITE_DIR.is_dir():
        raise SystemExit("site directory does not exist; run the docs build first")

    html_files = sorted(SITE_DIR.rglob("*.html"))
    if not html_files:
        raise SystemExit("no generated HTML files found under site/")

    errors: list[str] = []
    descriptions: dict[Path, str] = {}

    for html_file in html_files:
        parser = HtmlHeadParser()
        parser.feed(html_file.read_text(encoding="utf-8"))
        is_404 = html_file == SITE_DIR / "404.html"

        title = parser.title.strip()
        if not title:
            errors.append(f"{html_file}: missing <title>")
        elif not is_404 and (len(title) < 30 or len(title) > 70):
            errors.append(
                f"{html_file}: title length {len(title)} outside 30-70 chars"
            )

        if not parser.meta_description:
            errors.append(f"{html_file}: missing meta description")
        else:
            descriptions[html_file] = parser.meta_description.strip()

        if not is_404 and not parser.canonical:
            errors.append(f"{html_file}: missing canonical link")
        elif parser.canonical and not parser.canonical.startswith(EXPECTED_ORIGIN):
            errors.append(f"{html_file}: canonical does not use {EXPECTED_ORIGIN}")

        if html_file.name == "index.html" and not is_404 and parser.json_ld_blocks == 0:
            errors.append(f"{html_file}: missing JSON-LD structured data")

        for link in parser.links:
            if not link or link.startswith(
                ("http://", "https://", "mailto:", "tel:", "javascript:", "#")
            ):
                continue
            target = resolve_relative_target(html_file, link)
            if not target.exists():
                errors.append(f"{html_file}: broken relative link {link} -> {target}")

    duplicate_descriptions = [
        description
        for description, count in Counter(descriptions.values()).items()
        if count > 1
    ]
    if duplicate_descriptions:
        for description in duplicate_descriptions:
            pages = [str(path) for path, value in descriptions.items() if value == description]
            errors.append(
                "duplicate meta description used by: " + ", ".join(sorted(pages))
            )

    robots = SITE_DIR / "robots.txt"
    if not robots.exists():
        errors.append("site/robots.txt is missing")
    else:
        robots_text = robots.read_text(encoding="utf-8")
        expected_sitemap = f"Sitemap: {EXPECTED_ORIGIN}/sitemap.xml"
        if expected_sitemap not in robots_text:
            errors.append("site/robots.txt is missing the sitemap reference")

    llms = SITE_DIR / "llms.txt"
    if not llms.exists():
        errors.append("site/llms.txt is missing")

    if errors:
        for error in errors:
            print(f"ERROR: {error}")
        return 1

    print(f"Validated {len(html_files)} HTML files in {SITE_DIR}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
