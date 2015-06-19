Usage
-----

```zsh
$ # after cloning it
$ cargo build --release
$ # copy or link ./target/release/git-what-did-i to $PATH
$ # cd to your git repository
$ git what-did-i
JIRA URL: <put the root url of your JIRA service>
Username: <your account name>
Password: <and so on>
  bugfix/AD-224 	Exception on /admin/users [POST]
  feature/CL-337 	Add a new fascinate feature
  bugfix/MM-550 	Response is too slow on /data/<user_id>
* master
  improve/TR-736 	Rewrite app/trashes.js
```

Configurations will be stored to `.gitconfig` in your home directory, so you don't need to type your username and password again.
