alter table versions
    drop column semver_no_prerelease;

drop function to_semver_no_prerelease(text);
