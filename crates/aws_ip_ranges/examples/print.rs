fn main() {
    println!("CloudFront IP ranges:");
    for cidr in aws_ip_ranges::CLOUDFRONT_CIDRS {
        println!("  {}", cidr);
    }
}
