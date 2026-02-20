fn main() {
    println!("CloudFront IP ranges:");
    for cidr in crates_io_real_ip::CLOUDFRONT_NETWORKS {
        println!("  {}", cidr);
    }

    println!();

    println!("Fastly IP ranges:");
    for cidr in crates_io_real_ip::FASTLY_NETWORKS {
        println!("  {}", cidr);
    }
}
