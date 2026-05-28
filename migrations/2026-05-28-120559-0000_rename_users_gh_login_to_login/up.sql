alter table users rename column gh_login to login;
alter index lower_gh_login rename to lower_login;
