export DEBIAN_FRONTEND=noninteractive
#Update the local package database
apt-get update

#Install node
su vagrant -c 'curl -s -o- https://raw.githubusercontent.com/creationix/nvm/v0.33.8/install.sh | bash'
su vagrant -c 'export NVM_DIR="$HOME/.nvm"; [ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"; nvm install node'

#Install everything for the frontend with 'npm install'
su vagrant -c 'cd /vagrant; export NVM_DIR="$HOME/.nvm"; [ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"; nvm use node; npm install'

#Install Rust
su vagrant -c 'curl -s https://sh.rustup.rs -sSf > tmp.sh'
su vagrant -c 'chmod 775 tmp.sh'
su vagrant -c './tmp.sh -y'
rm tmp.sh

#Add .cargo/bin to path
grep 'export PATH=$PATH:.cargo/bin' /home/vagrant/.bashrc || echo 'export PATH=$PATH:.cargo/bin' | tee -a /home/vagrant/.bashrc

#Install Postgres
apt-get install -y postgresql postgresql-contrib libpq-dev pkg-config

#Make root a postgres superuser
if [ ! $(su postgres -c 'psql postgres -tAc "SELECT 1 FROM pg_roles WHERE rolname='"'"'root'"'"'"') ]
then
	echo "making root a PSQL user"
	su postgres -c 'createuser --superuser root'
fi

#Make vagrant a postgres superuser
if [ ! $(psql postgres -tAc "SELECT 1 FROM pg_roles WHERE rolname='vagrant'") ]
then
	echo "creating PSQL user 'vagrant'"
	su postgres -c 'createuser --superuser vagrant'
	su postgres -c 'createdb vagrant'
else
	echo "PSQL user 'vagrant' already exists"
fi

#Give psql user 'vagrant' a database
if [ ! $(psql postgres -tAc "select 1 from pg_database where datname='vagrant'") ]
then
	echo "creating PSQL database 'vagrant'"
	su postgres -c 'createdb vagrant'
else
	echo "PSQL database 'vagrant' already exists"
fi

#trust all local connections to psql.
#FIXME This is a security risk making this vagrant file only suitable
#for development.
#FIXME This will break when postgresql updates and this path changes.
sed -i 's/\(local\s*all.*\)peer/\1trust/' /etc/postgresql/9.5/main/pg_hba.conf
sed -i 's/\(host\s*all\s*all\s*.*\s*\)md5/\1trust/' /etc/postgresql/9.5/main/pg_hba.conf

#reload psql to load the above changes
/etc/init.d/postgresql reload

#Install cmake and libssl dependencies
apt-get install -y cmake
apt-get install -y libssl-dev pkg-config

#Install diesel-cli
if [ ! -e /home/vagrant/.cargo/bin/diesel ]
then
	echo "installing diesel: this may take a while"
	su - vagrant -c 'cargo install --quiet diesel_cli --no-default-features --features postgres'
else
	echo "diesel already installed"
fi

#Make the cargo_registry database
if [ ! $(psql postgres -tAc "select 1 from pg_database where datname='cargo_registry'") ]
then
	echo "creating PSQL DB 'cargo_registry'"
	su vagrant -c 'createdb cargo_registry'
else
	echo "PSQL database 'cargo_registry' already exists"
fi

#Run the diesel migrations and init the repos in ./tmp
echo "DEBUG: running migrations"
su - vagrant -c 'cd /vagrant; diesel migration run'
echo "DEBUG: initting local indices"
su - vagrant -c 'cd /vagrant; ./script/init-local-index.sh'
