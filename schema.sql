drop table if exists mail;
create table mail(
  mail_id integer not null primary key,
  request_id integer not null,
  creation_time integer not null,
  topic varchar(100) not null,
  destination varchar(100) not null,
  title varchar(100) not null,
  content varchar(100) not null
);
