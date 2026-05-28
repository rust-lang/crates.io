alter index lower_login rename to lower_gh_login;
alter table users rename column login to gh_login;
