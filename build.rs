use std::env;

fn main() {
    let key = "TORCH_CUDA_VERSION";
    env::set_var(key, "cu117");
    assert_eq!(env::var(key), Ok("cu117".to_string()));
}