use crate::simplify_bf;

pub fn pad_brainfuck_with_comments(code: String, comment: &str, desired_size: usize) -> String {
    let original_size = code.len();
    
    if original_size >= desired_size {
        return code; // No padding needed
    }

    let comment = if comment.is_empty() {
        "@%*".to_string()
    } else {
        comment.to_string()
    };

    let remaining_space = desired_size - original_size;
    let comment_with_spaces = format!("{}", comment); // Space before comment for readability
    let comment_size = comment_with_spaces.len();

    let num_insertions = remaining_space / comment_size;
    if num_insertions == 0 {
        // If there's no space for full comments, just pad with spaces
        return format!("{:<width$}", code, width = desired_size);
    }

    // Calculate evenly spaced insertion points
    let interval = (original_size + num_insertions) / num_insertions;
    let mut padded_code = String::new();
    let mut bf_chars = code.chars().peekable();
    let mut comment_count = 0;
    let mut total_length = 0;

    while total_length < desired_size {
        if total_length % interval == 0 && comment_count < num_insertions {
            // Insert a comment at the interval
            padded_code.push_str(&comment_with_spaces);
            total_length += comment_size;
            comment_count += 1;
        }

        // Insert Brainfuck code character if available
        if let Some(c) = bf_chars.next() {
            padded_code.push(c);
            total_length += 1;
        } else {
            break;
        }
    }

    // Final padding with spaces if needed
    while padded_code.len() < desired_size {
        padded_code.push(' ');
    }

    padded_code.truncate(desired_size); // Ensure exact size
    padded_code
}
