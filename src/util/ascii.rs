use crate::simplify_bf;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::path::Path;

use tracing::*;

const PLACEHOLDER: char = '#';

pub fn scale_ascii_art(art: &str, scale_factor: usize) -> String {
    let lines: Vec<&str> = art.lines().collect();
    let mut scaled_art = Vec::new();

    for line in &lines {
        let scaled_line: String = line.chars()
            .flat_map(|c| std::iter::repeat(c).take(scale_factor)) // Scale horizontally
            .collect();
        
        for _ in 0..scale_factor { // Scale vertically
            scaled_art.push(scaled_line.clone());
        }
    }

    scaled_art.join("\n")
}

pub fn ascii_art_size(art: &str) -> (usize, usize) {
    let lines: Vec<&str> = art.lines().collect();
    let width = lines.iter().map(|line| line.chars().count()).max().unwrap_or(0);
    let height = lines.len();
    (width, height)
}

/// Make the ascii art fill the given size with whitespace where needed
pub fn ascii_art_fill(art: &str, width: usize, height: usize) -> String {
    let lines: Vec<&str> = art.lines().collect();
    let mut filled_art = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let mut filled_line = line.to_string();
        if filled_line.len() < width {
            filled_line.push_str(&" ".repeat(width - filled_line.len()));
        }
        filled_art.push(filled_line);
    }

    while filled_art.len() < height {
        filled_art.push(" ".repeat(width));
    }

    filled_art.join("\n")
}

pub fn replace_brainfuck_chars(mut art: String) -> String {
    // For every non-whitespace character, replace it with a placeholder
    art = art.replace('-', "~");
    art = art.replace('+', "*");
    art = art.replace('.', ":");
    art = art.replace(',', ";");
    art = art.replace('#', "@");
    art = art.replace('$', "S");
    art = art.replace('>', "/");
    art = art.replace('<', "\\");
    art = art.replace('[', "{");
    art = art.replace(']', "}");
    art
}

fn available_brainfuck_slots(art: &str) -> usize {
    art.chars().filter(|c| !c.is_whitespace()).count()
}

pub fn apply_ascii_art_template(art_template: &str, bf: String, comment: &str) -> String {
    let mut art_template = replace_brainfuck_chars(art_template.to_string());
    let (width, height) = ascii_art_size(art_template.as_str());
    art_template = ascii_art_fill(&art_template, width, height);
    let mut bf = simplify_bf(bf);

    while available_brainfuck_slots(&art_template) < bf.len() {
        // Scale the art
        art_template = scale_ascii_art(&art_template, 2);
    }

    // Now, pad the bf with comments
    let desired_size = available_brainfuck_slots(&art_template);
    bf = super::bf::pad_brainfuck_with_comments(bf, comment, desired_size);
    debug!("Available slots: {}, bf len: {}", available_brainfuck_slots(&art_template), bf.len());

    // Go through every non-whitespace character in the template and replace it with a character from the brainfuck code
    let mut bf_iter = bf.chars();
    art_template = art_template.chars().map(|c| {
        if c.is_whitespace() {
            c
        } else {
            bf_iter.next().unwrap_or(PLACEHOLDER)
        }
    }).collect();

    assert!(bf_iter.next().is_none(), "Not enough space in art template for brainfuck code");

    art_template
}


lazy_static! {
    static ref ASCII_ART_TEMPLATES: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("radioactive", include_str!("./templates/radioactive.txt"));
        m.insert("lightbulb", include_str!("./templates/lightbulb.txt"));
        m.insert("peace", include_str!("./templates/peace.txt"));
        m.insert("coca-cola", include_str!("./templates/coca-cola.txt"));
        m.insert("bomb", include_str!("./templates/bomb.txt"));
        m.insert("revolver", include_str!("./templates/revolver.txt"));
        m.insert("smile", include_str!("./templates/smile.txt"));
        m.insert("mandelbrot", include_str!("./templates/mandelbrot.txt"));
        m.insert("tiny", include_str!("./templates/tiny.txt"));
        m.insert("footgun", include_str!("./templates/footgun.txt"));
        m.insert("adam", include_str!("./templates/adam.txt"));
        m.insert("jolly-roger", include_str!("./templates/jolly-roger.txt"));
        m.insert("cigarette", include_str!("./templates/cigarette.txt"));
        m
    };
}

pub fn get_template_names() -> Vec<&'static str> {
    ASCII_ART_TEMPLATES.keys().cloned().collect()
}

pub fn check_valid_template(name: &str) -> bool {
    if ASCII_ART_TEMPLATES.contains_key(name) {
        return true;
    }
    // Open it from the file system
    let path = Path::new(name);
    if !path.exists() {
        error!("Template file not found: {}", name);
        error!("Try choosing from one of the following:");
        for template in get_template_names() {
            error!("• {}", template);
        }
        return false;
    }
    true
}

pub fn apply_template_from_name_or_file(name: &str, bf: String, comment: Option<&str>) -> std::io::Result<String> {
    let comment = comment.unwrap_or("");
    let template = match ASCII_ART_TEMPLATES.get(name) {
        Some(template) => {
            info!("Using template: {}", name);
            template.to_string()
        },
        None => {
            // Open it from the file system
            let path = Path::new(name);
            check_valid_template(name);
            let template_file = std::fs::read_to_string(path)?;
            template_file
        }
    };

    let art = apply_ascii_art_template(&template, bf, comment);
    Ok(art)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    #[test]
    fn test_apply_template() {
        init_logging();

        // Compile the following into BF:
        let asm = "main:
            putchar 'F'
            putchar 'a'
            putchar 'c'
            putchar 't'
            putchar ' '
            putchar 'o'
            putchar 'f'
            putchar ' '
            R0 = 5
            putint R0
            putchar ':'
            putchar ' '
            push R0
            call fact
            putint [SP]
            putchar '\n'
            quit

        fact:
            R0 eq [SP], 1
            jmp_if R0, end

            push [SP]
            dec [SP]
            
            call fact
            pop R0
            [SP] mul R0
            ret
        end:
            [SP] = 1
            ret";
        let bf = crate::Program::parse(asm).unwrap().assemble();
        let comment = "adam mcdaniel is cool";
    }
}