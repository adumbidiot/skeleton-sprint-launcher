fn main() {
    let sdk_loc_env = std::env::var_os("STEAM_SDK_LOCATION").expect("STEAM_SDK_LOCATION");
    let sdk_loc = std::path::Path::new(&sdk_loc_env);
    cc::Build::new()
        .cpp(true)
        .include(sdk_loc.join("public/steam"))
        .file("src/cpp/steamworks_rs_patches.cpp")
        .compile("steamworks_rs_patches");
}
