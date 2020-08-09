

struct ScpiFramer {
    string_char: u8,
    in_string: bool,
    arbitrary_len: usize,
    arbitrary_header_len: Option<u8>,
    in_arbitrary: bool,
    in_arbitrary_header: bool
}

impl ScpiFramer {

    pub fn digest(&mut self, char: u8) -> bool {
        if self.in_string {
            if char == self.string_char {
                self.in_string = false;
            }
            false
        }else if self.in_arbitrary {
            false
        }else {
            if char == b'\'' || char == b'"' {
                self.string_char = char;
                self.in_string = true;
            }
            if char == b'#' {
                self.in_arbitrary_header = true;
            }
            else if self.in_arbitrary_header && self.arbitrary_header_len.is_none(){
                if char.is_ascii_digit() {

                }
            }
            false
        }
    }
}