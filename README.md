# Relai libp2p
Ce code vous permet de lancer votre propre nœud de relai libp2p pour vos applications basées sur SFIP ou OrbitDB.

## Installation
En premier lieu, fourchez ce projet et installez-le localement :

```sh
pnpm install
```

## Déploiement
Pour déployer votre propre serveur relai, suivre les instructions suivantes :

### Choisissez un fournisseur
Vous devrez choisir un fournisseur d'hébergement. [Back4App Containers](https://www.back4app.com/docs-containers) offre des options gratuites qui sont suffisamment puissantes pour ce relai. [Heroku](https://devcenter.heroku.com/articles/github-integration) offre également des options payantes.

Créez votre compte sur le site de votre fournisseur, puis choisissez de créer une nouvelle application avec l'addresse GitHub de la fourche de ce projet que vous avez créée ci-dessus.

### Génération de la clef secrète
1. Exécutez `pnpm lancer`.
2. Ouvrez le fichier `.env` qui vient d'être généré et copiez la valeur **secrète** dans les variables d'environnement ([Back4App](https://www.back4app.com/docs-containers/prepare-your-deployment), [Heroku](https://devcenter.heroku.com/articles/config-vars)) de votre fournisseur d'hébergement.

### Configuration du domaine
1. Choisissez un nom de domaine pour votre serveur relai. Ceci peut être un nom par défaut de votre fournisseur (p. ex., monprojet.b4a.run ou monprojet.herokuapp.com) ou bien votre propre domaine.
2. Copiez ce nom de domaine dans une variable d'environnement nommée `DOMAINE` sur votre fournisseur d'hébergement.

## Crédits
Beaucoup du code provient originalement de https://github.com/libp2p/js-libp2p-example-circuit-relay.

# 
https://docs.digitalocean.com/products/droplets/getting-started/recommended-droplet-setup/

https://www.digitalocean.com/community/tutorials/initial-server-setup-with-ubuntu-20-04

https://www.digitalocean.com/community/tutorials/how-to-install-nginx-on-ubuntu-20-04#step-5-%E2%80%93-setting-up-server-blocks-(recommended)

https://www.digitalocean.com/community/tutorials/how-to-secure-nginx-with-let-s-encrypt-on-ubuntu-20-04

https://www.digitalocean.com/community/tutorials/how-to-set-up-a-node-js-application-for-production-on-ubuntu-20-04

pm2 start pnpm --name serveur  -- lancer


/etc/nginx/sites-available/MON_DOMAINE
```
server {
        server_name relai-ws-libp2p.xn--rseau-constellation-bzb.ca;
        location / {
                proxy_pass http://localhost:12345;
                proxy_http_version 1.1;
                proxy_set_header Upgrade $http_upgrade;
                proxy_set_header Connection 'upgrade';
                proxy_set_header Host $host;
                proxy_cache_bypass $http_upgrade;
       }
        location /info/ {
                proxy_pass http://localhost:8000/;
                proxy_http_version 1.1;
                proxy_set_header Upgrade $http_upgrade;
                proxy_set_header Connection 'upgrade';
                proxy_set_header Host $host;
                proxy_cache_bypass $http_upgrade;
        }
        location /adresses/ {
                proxy_pass http://localhost:8000;
                proxy_http_version 1.1;
                proxy_set_header Upgrade $http_upgrade;
                proxy_set_header Connection 'upgrade';
                proxy_set_header Host $host;
                proxy_cache_bypass $http_upgrade;
        }

    listen [::]:443 ssl ipv6only=on; # managed by Certbot
    listen 443 ssl; # managed by Certbot
    ssl_certificate /etc/letsencrypt/live/relai-ws-libp2p.xn--rseau-constellation-bzb.ca/fullchain.pem; # managed by Certbot
    ssl_certificate_key /etc/letsencrypt/live/relai-ws-libp2p.xn--rseau-constellation-bzb.ca/privkey.pem; # managed by Certbot
    include /etc/letsencrypt/options-ssl-nginx.conf; # managed by Certbot
    ssl_dhparam /etc/letsencrypt/ssl-dhparams.pem; # managed by Certbot

}
server {
    if ($host = relai-ws-libp2p.xn--rseau-constellation-bzb.ca) {
        return 301 https://$host$request_uri;
    } # managed by Certbot


        listen 80;
        listen [::]:80;

        server_name relai-ws-libp2p.xn--rseau-constellation-bzb.ca;
    return 404; # managed by Certbot


}

```