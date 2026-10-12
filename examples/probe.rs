use hidapi::HidApi;

fn main() {
    let api = HidApi::new().expect("Failed to init hidapi");
    println!("Scanning devices...");
    for dev in api.device_list() {
        if dev.vendor_id() == 0x3554 || dev.vendor_id() == 0x258a {
            println!(
                "Found: VID={:04x} PID={:04x} Path={:?} Interface={} UsagePage=0x{:04x} Usage=0x{:04x}",
                dev.vendor_id(),
                dev.product_id(),
                dev.path(),
                dev.interface_number(),
                dev.usage_page(),
                dev.usage()
            );
            match dev.open_device(&api) {
                Ok(_) => println!("  -> Successfully OPENED!"),
                Err(e) => println!("  -> Failed to open: {}", e),
            }
        }
    }
}
