// =============================================================================
// CRATES.IO OG-IMAGE TEMPLATE
// =============================================================================
// This template generates Open Graph images for crates.io crate.

// =============================================================================
// COLOR PALETTE
// =============================================================================

#let colors = (
    bg: oklch(97%, 0.0147, 98deg),
    rust-overlay: oklch(36%, 0.07, 144deg, 20%),
    header-bg: oklch(36%, 0.07, 144deg),
    header-text: oklch(100%, 0, 0deg),
    primary: oklch(36%, 0.07, 144deg),
    text: oklch(51%, 0.05, 144deg),
    text-light: oklch(60%, 0.05, 144deg),
    avatar-bg: oklch(100%, 0, 0deg),
    avatar-border: oklch(87%, 0.01, 98deg),
    tag-bg: oklch(36%, 0.07, 144deg),
    tag-text: oklch(100%, 0, 0deg),
)

// =============================================================================
// LAYOUT CONSTANTS
// =============================================================================

#let header-height = 60pt
#let footer-height = 4pt

// =============================================================================
// TEXT TRUNCATION UTILITIES
// =============================================================================
// These functions handle text overflow by adding ellipsis when content
// exceeds specified dimensions

// Truncates text to fit within a maximum height
// @param text: The text content to truncate
// @param maxHeight: Maximum height constraint (optional, defaults to single line height)
#let truncate_to_height(text, maxHeight: none) = {
    layout(size => {
        let text = text

        let maxHeight = if maxHeight != none {
            maxHeight
        } else {
            measure(text).height
        }

        if measure(width: size.width, text).height <= maxHeight {
            return text
        } else {
            while measure(width: size.width, text + "…").height > maxHeight {
                // Use character-based slicing instead of byte-based to handle Unicode correctly
                let chars = text.clusters()
                if chars.len() == 0 {
                    break
                }
                text = chars.slice(0, chars.len() - 1).join().trim()
            }
            return text + "…"
        }
    })
}

// Truncates text to fit within a maximum width
// @param text: The text content to truncate
// @param maxWidth: Maximum width constraint (optional, defaults to container width)
#let truncate_to_width(text, maxWidth: none) = {
    layout(size => {
        let text = text

        let maxWidth = if maxWidth != none {
            maxWidth
        } else {
            size.width
        }

        if measure(text).width <= maxWidth {
            return text
        } else {
            while measure(text + "…").width > maxWidth {
                // Use character-based slicing instead of byte-based to handle Unicode correctly
                let chars = text.clusters()
                if chars.len() == 0 {
                    break
                }
                text = chars.slice(0, chars.len() - 1).join().trim()
            }
            return text + "…"
        }
    })
}

// =============================================================================
// IMAGE UTILITIES
// =============================================================================
// Functions for loading and processing images

// Loads an SVG icon and replaces currentColor with the specified color
// @param icon-name: The name of the SVG file (without .svg extension)
// @param color: The color to replace currentColor with
// @param width: The width of the image (default: auto)
// @param height: The height of the image (default: auto)
#let colored-image(path, color, width: auto, height: auto) = {
    let svg = read(path).replace("currentColor", color.to-hex())
    image(bytes(svg), width: width, height: height)
}

// =============================================================================
// AVATAR RENDERING
// =============================================================================
// Functions for rendering circular avatar images

// Renders a circular avatar image with border
// @param avatar-path: Path to the avatar image file
// @param size: Size of the avatar (default: 1em)
#let render-avatar(avatar-path, size: 1em) = {
    box(clip: true, fill: colors.avatar-bg, stroke: 0.5pt + colors.avatar-border,
        radius: 50%, inset: 1pt,
        box(clip: true, radius: 50%, image(avatar-path, width: size))
    )
}

// =============================================================================
// AUTHOR HANDLING
// =============================================================================
// Complex logic for displaying multiple authors with proper grammar

// Renders an author with optional avatar and name
// @param author: Object with 'name' and optional 'avatar' properties
#let render-author(author) = {
    if author.avatar != none {
        h(0.2em)
        box(baseline: 30%, [#render-avatar(author.avatar, size: 1.5em)])
        h(0.2em)
    }
    author.name
}

// Generates grammatically correct author list text
#let generate-authors-text(authors, maxVisible: none) = {
    if authors.len() == 0 {
        return ""
    }

    let prefix = "by "
    let visible = if maxVisible != none {
        calc.min(maxVisible, authors.len())
    } else {
        authors.len()
    }

    if authors.len() == 1 {
        return prefix + render-author(authors.at(0))
    }

    // Build the visible authors list
    let authors-text = ""
    for i in range(visible) {
        if i == 0 {
            authors-text += render-author(authors.at(i))
        } else if i == visible - 1 and visible == authors.len() {
            // Last author and we're showing all authors
            authors-text += " and " + render-author(authors.at(i))
        } else {
            // Not the last author, or we're truncating
            authors-text += ", " + render-author(authors.at(i))
        }
    }

    // Add "and X others" suffix if truncated
    if visible < authors.len() {
        let remaining = authors.len() - visible
        let suffix = " and " + str(remaining) + " other"
        if remaining > 1 {
            suffix += "s"
        }
        authors-text += suffix
    }

    return prefix + authors-text
}

// Renders authors list with intelligent truncation based on available width
#let render-authors-list(authors, maxWidth: none) = {
    layout(size => {
        let maxWidth = if maxWidth != none {
            maxWidth
        } else {
            size.width
        }

        if authors.len() == 0 {
            return ""
        }

        // Try showing all authors first
        let full-text = generate-authors-text(authors)
        if measure(full-text).width <= maxWidth {
            return full-text
        }

        // Reduce maxVisible until text fits
        let maxVisible = authors.len() - 1
        while maxVisible >= 1 {
            let truncated-text = generate-authors-text(authors, maxVisible: maxVisible)
            if measure(truncated-text).width <= maxWidth {
                return truncated-text
            }
            maxVisible -= 1
        }

        // Fallback: just show first author and "and X others"
        return generate-authors-text(authors, maxVisible: 1)
    })
}

// =============================================================================
// VISUAL COMPONENTS
// =============================================================================
// Reusable components for consistent styling

#let render-header = {
    rect(width: 100%, height: header-height, fill: colors.header-bg, {
        place(left + horizon, dx: 30pt, {
            box(baseline: 30%, image("assets/cargo.png", width: 35pt))
            h(10pt)
            text(size: 22pt, fill: colors.header-text, weight: "semibold")[crates.io]
        })
    })
}

// Renders a tag/keyword with consistent styling
#let render-tag(content) = {
    set text(fill: colors.tag-text)
    box(fill: colors.tag-bg, radius: .15em, inset: (x: .4em, y: .25em),
        content
    )
}

// Renders a metadata item with icon, title, and content
#let render-metadata(title, content, icon-name) = {
    let icon-path = "assets/" + icon-name + ".svg"

    box(inset: (right: 20pt),
        grid(columns: (auto, auto), rows: (auto, auto), column-gutter: .75em, row-gutter: .5em,
            grid.cell(rowspan: 2, align: horizon, colored-image(icon-path, colors.primary, height: 1.2em)),
            text(size: 8pt, fill: colors.text-light, upper(title)),
            text(size: 12pt, fill: colors.primary, content)
        )
    )
}

// =============================================================================
// DATA LOADING
// =============================================================================
// Load data from sys.inputs

#let data = json(bytes(sys.inputs.data))
#let avatar_map = json(bytes(sys.inputs.at("avatar_map", default: "{}")))

// =============================================================================
// MAIN DOCUMENT
// =============================================================================

#set page(width: 600pt, height: 315pt, margin: 0pt, fill: colors.bg)
#set text(font: "Fira Sans", fill: colors.text)

// Header with crates.io branding
#render-header

// Bottom border accent
#place(bottom,
    rect(width: 100%, height: footer-height, fill: colors.header-bg)
)

// Rust logo overlay (20% opacity watermark)
#place(bottom + right, dx: 200pt, dy: 100pt,
    colored-image("assets/rust-logo.svg", colors.rust-overlay, width: 300pt)
)

// Main content area
#place(
    left + top,
    dy: 60pt,
    block(height: 100% - header-height - footer-height, inset: 35pt, clip: true, {
        // Crate name
        block(text(size: 36pt, weight: "semibold", fill: colors.primary, truncate_to_width(data.name)))

        // Tags
        if data.at("tags", default: ()).len() > 0 {
            block(
                for (i, tag) in data.tags.enumerate() {
                    if i > 0 {
                        h(3pt)
                    }
                    render-tag(text(size: 8pt, weight: "medium", "#" + tag))
                }
            )
        }

        // Description
        if data.at("description", default: none) != none {
            block(text(size: 14pt, weight: "regular", truncate_to_height(data.description, maxHeight: 60pt)))
        }

        // Authors
        if data.at("authors", default: ()).len() > 0 {
            set text(size: 10pt, fill: colors.text-light)
            let authors-with-avatars = data.authors.map(author => {
                let avatar = none
                if author.avatar != none {
                    let avatar_path = avatar_map.at(author.avatar, default: none)
                    if avatar_path != none {
                        avatar = "assets/" + avatar_path
                    }
                }
                (name: author.name, avatar: avatar)
            })
            block(render-authors-list(authors-with-avatars))
        }

        place(bottom + left, float: true,
            stack(dir: ltr, {
                if data.at("releases", default: none) != none {
                    render-metadata("Releases", data.releases, "tag")
                }
                render-metadata("Latest", truncate_to_width("v" + data.version, maxWidth: 80pt), "code-branch")
                if data.at("license", default: none) != none {
                    render-metadata("License", truncate_to_width(data.license, maxWidth: 100pt), "scale-balanced")
                }
                if data.at("lines_of_code", default: none) != none {
                    render-metadata("SLoC", data.lines_of_code, "code")
                }
                if data.at("crate_size", default: none) != none {
                    render-metadata("Size", data.crate_size, "weight-hanging")
                }
            })
        )
    })
)
