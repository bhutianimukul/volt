//! Native macOS image file picker using NSOpenPanel.

/// Open a native file picker dialog filtered to image types.
/// Returns `Some(path)` if the user selected a file, `None` if cancelled.
#[cfg(target_os = "macos")]
pub fn pick_image() -> Option<String> {
    use objc::class;
    use objc::msg_send;
    use objc::runtime::Object;
    use objc::sel;
    use objc::sel_impl;

    const UTF8_ENCODING: usize = 4;

    unsafe {
        let panel: *mut Object = msg_send![class!(NSOpenPanel), openPanel];

        // Configure the panel
        let title = "Choose Background Image";
        let title_ns: *mut Object = msg_send![class!(NSString), alloc];
        let title_ns: *mut Object = msg_send![title_ns, initWithBytes:title.as_ptr()
            length:title.len()
            encoding:UTF8_ENCODING];
        let _: () = msg_send![panel, setTitle: title_ns];
        let _: () = msg_send![panel, setCanChooseFiles: true];
        let _: () = msg_send![panel, setCanChooseDirectories: false];
        let _: () = msg_send![panel, setAllowsMultipleSelection: false];

        // Filter to image types using UTType
        let extensions = ["png", "jpg", "jpeg", "gif", "bmp", "tiff", "webp", "heic"];
        let mut ns_extensions: Vec<*mut Object> = Vec::new();
        for ext in &extensions {
            let ns: *mut Object = msg_send![class!(NSString), alloc];
            let ns: *mut Object = msg_send![ns, initWithBytes:ext.as_ptr()
                length:ext.len()
                encoding:UTF8_ENCODING];
            ns_extensions.push(ns);
        }

        let ext_array: *mut Object = msg_send![class!(NSArray),
            arrayWithObjects:ns_extensions.as_ptr()
            count:ns_extensions.len()];
        let _: () = msg_send![panel, setAllowedFileTypes: ext_array];

        // Run the panel modally
        let response: isize = msg_send![panel, runModal];

        // NSModalResponseOK = 1
        if response == 1 {
            let url: *mut Object = msg_send![panel, URL];
            if !url.is_null() {
                let path: *mut Object = msg_send![url, path];
                if !path.is_null() {
                    let len: usize =
                        msg_send![path, lengthOfBytesUsingEncoding: UTF8_ENCODING];
                    let bytes: *const u8 = msg_send![path, UTF8String];
                    if !bytes.is_null() && len > 0 {
                        let slice = std::slice::from_raw_parts(bytes, len);
                        if let Ok(s) = std::str::from_utf8(slice) {
                            return Some(s.to_string());
                        }
                    }
                }
            }
        }

        None
    }
}

#[cfg(not(target_os = "macos"))]
pub fn pick_image() -> Option<String> {
    // On non-macOS platforms, fall back to text input
    None
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_pick_image_returns_option() {
        // Can't test the actual dialog in CI, but ensure the function signature works
        // On non-macOS or headless, this just returns None
        #[cfg(not(target_os = "macos"))]
        {
            assert!(super::pick_image().is_none());
        }
    }
}
