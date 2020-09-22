fn main() {
    let sdk_loc = std::env::var("STEAM_SDK_LOCATION").expect("Steam SDK");
    let sdk_loc = std::path::Path::new(&sdk_loc);
    cc::Build::new()
        .cpp(true)
        .include(sdk_loc.join("public/steam"))
        .file("patch.cpp")
        .compile("patch");
}
