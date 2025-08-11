<PageHeader @title='Security Information' />

<TextContent @boxed={{true}}>

  <h2 id='crates-io-security'>Security of crates.io itself</h2>

  <p>Safety is one of the core principles of Rust, and to that end, we would like to ensure that cargo, crates.io, docs.rs, and
    related tools have secure implementations. To disclose security vulnerabilities in the crates.io service itself (as opposed
    to crates hosted on crates.io) or any other <a href='https://github.com/rust-lang'>repository in the rust-lang
    organization</a>, please follow the <a href='https://www.rust-lang.org/policies/security'>Rust Security policy</a>.</p>

    <p>Thank you for taking the time to responsibly disclose any issues you find.</p>

  <h2 id='crate-security'>Security of crates hosted on crates.io</h2>

  <p>To disclose security vulnerabilities found in a crate that is hosted on crates.io, seek guidance from the individual crate's
    owners and their specific policies. Commonly, projects include a file named <code>SECURITY.md</code> that contains the
    crate's security policies and procedures.</p>

  <p>Intentionally malicious code is against <LinkTo @route="policies">crates.io's usage policies</LinkTo>; please report crates
    violating these policies to <a href="mailto:help@crates.io">help@crates.io</a>.</p>

  <h2 id='rustsec'>Rustsec Security Advisory Database for receiving security updates</h2>

  <p>The <a href="https://rustsec.org/">Rustsec Security Advisory Database</a> maintains advisories about vulnerabilities in
    crates published on crates.io. Maintained by the <a href="https://www.rust-lang.org/governance/wgs/wg-secure-code">Secure
    Code Working Group</a>, the information is available in a variety of forms to incorporate into your development practices.
    See <a href="https://rustsec.org/contributing.html">their steps to submit a vulnerability to the database</a>.</p>

  <h2 id='ecosystem-security-help'>Ecosystem security help for crate authors</h2>

  <p>Security is a value important to the Rust ecosystem as a whole, not just to the Rust language. If you are a crate author and
    you have received a high impact/severity security bug report for your crate, the Rust Foundation and the Rust Project are
    available to help manage the situation. The Rust Project or the Rust Foundation may also be the ones reaching out to you, if
    they have been informed of a security issue.</p>

  <p>As part of its <a href="https://foundation.rust-lang.org/tags/security%20initiative/">Security Initiative</a>, the Rust
    Foundation:</p>

  <ul>
    <li>Employs security engineers who can help assessing the problem, developing mitigations, and estimating impact.</li>
    <li>Has a network of member organizations that can help with testing resources and also employ security experts who can help
      with assessing and fixing issues.</li>
    <li>Employs communications staff who can manage publishing notifications and fielding inquiries.</li>
    <li>Has contacts with government agencies tasked with cybersecurity protections who may have information on exploitation or
      impact of a security problem.</li>
  </ul>

  <p>The Rust Project can coordinate actions among other parts of the ecosystem that may need to be updated to address a fix.</p>

  <p>Please reach out to <a href="mailto:contact@rustfoundation.org">contact@rustfoundation.org</a> if either the Rust Project or
    the Rust Foundation can help you by providing security support in the areas listed above or in another way! These are just a
    few examples of the kind of help available to crate authors facing security challenges.</p>

</TextContent>
