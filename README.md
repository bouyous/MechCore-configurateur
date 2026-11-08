# Configurateur de firmware MeshCore

Application de bureau en français pour préparer, personnaliser, compiler et flasher un firmware MeshCore depuis ses sources officielles. Interface Rust avec egui/eframe, fond bleu, navigation latérale et bandeau LoRa.

Projet indépendant : il ne s’agit pas d’un outil officiel MeshCore, ExpressLRS ou Semtech.

**Auteur** — Bouyou. Développé avec l’assistance de Codex.

**Phase expérimentale.** Le flash peut effacer les données ou rendre une carte inutilisable. Sauvegardez la configuration et l’identité de la carte, puis vérifiez le modèle et le rôle avant de lancer le transfert. Le logiciel est fourni en l’état, sans garantie. Dans les limites du droit applicable, Bouyou décline toute responsabilité pour les dommages résultant du flash. Voir la [licence MIT](LICENSE).

## Fonctions

- Actualisation des versions depuis GitHub, choix séparé du numéro de version et du rôle du firmware, téléchargement des sources ou import d’un dossier local.
- Recherche de cartes et sélection d’une cible parmi les environnements PlatformIO des sources chargées.
- Options de surveillance batterie (seuil bas, seuil de reprise, intervalle), de télémétrie publique, de journaux et d’annonce au démarrage, selon la compatibilité des sources.
- Configuration de quatre GPIO au maximum : entrées, pull-up, niveaux de sortie au démarrage et droits de lecture.
- Catalogue recherchable de périphériques et plans de câblage GPIO, UART, I²C, SPI, ADC et 1-Wire, avec liens de documentation.
- Bot météo BME280 facultatif : messages programmés, canaux publics, température, pression et minima/maxima.
- Enregistrement, import et export de profils JSON, journal des opérations et historique des compilations.
- Compilation dans une copie séparée des sources et transfert par PlatformIO sur un port série USB Windows, avec effacement facultatif après confirmation.

## Installer et lancer sous Windows

La version actuelle utilise Windows PowerShell pour les consultations GitHub. Les autres systèmes ne sont pas validés.

Pour construire l’application, installer Rust avec Cargo et les outils de compilation C++ Windows nécessaires à Rust. Pour télécharger et compiler les firmwares, installer aussi Git et PlatformIO Core ; `git` et `pio` doivent être accessibles dans le PATH. Le chemin de l’exécutable PlatformIO est modifiable dans « Compiler et flasher ». Une connexion Internet est nécessaire pour les téléchargements ; les pilotes USB dépendent de la carte.

Dans le dossier contenant `Cargo.toml` :

```powershell
cargo build --release --locked
cargo run --release --locked
```

L’exécutable est produit dans `target/release/mesh-studio.exe`. Le dossier `data/` est créé dans le dossier du projet enregistré lors de la compilation : cette version doit donc être utilisée avec ce dossier accessible en écriture. Déplacer uniquement l’exécutable ne constitue pas une installation portable.

## Préparer un firmware

1. Dans « Configurateur », actualiser les versions, choisir la version et le type de firmware, puis télécharger les sources. Un dossier MeshCore local peut aussi être importé.
2. Choisir la carte et sa cible, puis régler les options souhaitées.
3. Si des périphériques sont utilisés, vérifier le schéma constructeur et les broches. Appliquer les fonctions intégrées souhaitées ; confirmer le câblage lorsqu’il est demandé.
4. Dans « Compiler et flasher », vérifier le récapitulatif et lancer la compilation. Les fichiers `.bin`, `.uf2` ou `.hex` disponibles et le reçu `build.json` sont copiés dans `data/builds/<identifiant>/firmware/`.
5. Sauvegarder les données de la carte, vérifier que la compilation lui correspond, indiquer le port série et confirmer avant de flasher. L’option d’effacement supprime les données de l’appareil.

Un profil JSON décrit une configuration : ce n’est pas un firmware à flasher.
Les profils échangés ne choisissent ni l’exécutable PlatformIO, ni le dossier source, ni le correctif, ni le port de cette installation. Sélectionnez les sources à nouveau après l’import.

PlatformIO exécute les scripts des sources, correctifs et dépendances avec les droits de votre compte Windows. La copie des sources protège les originaux des modifications faites par le configurateur. Compilez uniquement des sources et correctifs de confiance, sans lancer l’application en administrateur.

### USB, Bluetooth et Wi-Fi

Le bouton de flash utilise un port série USB de type `COM3`. Les choix Bluetooth et Wi-Fi servent à compiler un firmware Compagnon avec l’interface correspondante, si la cible la propose. Le configurateur ne détecte pas d’appareils sans fil et ne transfère aucun firmware par Bluetooth ou Wi-Fi.

Pour une carte et un firmware compatibles OTA, suivez les procédures MeshCore pour le [transfert Wi-Fi sur ESP32](https://github.com/meshcore-dev/MeshCore/blob/main/docs/faq.md#72-q-how-to-update-esp32-based-devices-over-the-air) et le [DFU Bluetooth sur nRF52](https://github.com/meshcore-dev/MeshCore/blob/main/docs/faq.md#71-q-how-to-update-nrf-rak-t114-seed-xiao-repeater-and-room-server-firmware-over-the-air-using-the-new-simpler-dfu-app). Les formats de fichier et prérequis varient selon la carte.

## Compatibilité et limites

Les modifications sont appliquées aux sources puis compilées, sans décompilation de binaires. Leur compatibilité est contrôlée pendant la préparation ; toutes les versions et toutes les cartes ne sont pas garanties. Les GPIO et le bot BME280 intégrés sont destinés aux cibles répéteur ESP32 compatibles. Un correctif Git personnalisé peut être fourni, mais ne peut pas être combiné avec ces fonctions intégrées.

L’option de télémétrie du répéteur filtre les canaux accessibles aux invités après la connexion MeshCore habituelle. Les coordonnées GPS sont exclues des réponses aux invités.

Les autres périphériques du catalogue disposent d’un plan de câblage enregistré dans le profil ; leur sélection n’installe pas de pilote et ne les active pas dans le firmware. Les lignes proposées ne garantissent pas qu’une broche soit exposée ou libre sur chaque révision de carte. Voir [HARDWARE.md](HARDWARE.md) pour les sources et limites du brochage.

La surveillance batterie ne peut pas rallumer un appareil physiquement privé d’alimentation ou coupé par sa protection électrique. Les seuils, le redémarrage, la radio et la consommation doivent être vérifiés sur le matériel avant une installation distante. Les essais matériels et le flash réel ne sont pas validés par les tests logiciels.

## Vérifications et organisation

```powershell
cargo test --locked
cargo build --release --locked
```

Un test de compilation firmware est disponible avec un dossier MeshCore de confiance et PlatformIO dans le PATH :

```powershell
$env:MESH_TEST_SOURCE = 'C:\chemin\vers\MeshCore'
$env:MESH_TEST_TARGET = 'Xiao_S3_WIO_companion_radio_ble'
cargo test --locked builds_meshcore_without_flashing -- --ignored --nocapture
```

Un contrôle C++ du bot peut également être exécuté si Clang est installé :

```powershell
clang++ -std=c++17 -DESP32 -I tests/bot_stubs tests/bot_runtime.cpp -o target/bot-runtime.exe
.\target\bot-runtime.exe
```

- `src/main.rs` : interface et navigation.
- `src/firmware.rs` : profils, validation, téléchargement, modifications, compilation et flash.
- `src/hardware.rs` : catalogue, brochage et vérification des broches.
- `src/atelier_bot.h` : bot embarqué facultatif.
- `tests/` : contrôle natif C++ et ses substituts matériels ; les tests Rust se trouvent aussi dans les modules `src/`.
- `data/` : profils, cache de versions, sources et compilations locales ; exclu du dépôt.

## Mettre le projet sur GitHub

Utiliser le contenu de ce dossier `app/` comme racine du dépôt : inclure `src/`, `tests/`, `Cargo.toml`, `Cargo.lock`, `README.md`, `HARDWARE.md` et `.gitignore`. Le fichier `.gitignore` exclut les données personnelles, les sources téléchargées et les binaires générés. Vérifier manuellement les fichiers ajoutés, surtout les exports de profils et leurs chemins locaux.

Le configurateur est distribué sous [licence MIT](LICENSE). Les sources MeshCore téléchargées conservent leurs propres licences. Toute redistribution de firmware doit en respecter les conditions. [MeshCore](https://github.com/meshcore-dev/MeshCore) est le projet amont. LoRa est une marque de Semtech.

Pour contribuer, décrire le problème ou la modification, préciser la carte et la version concernées, et joindre les résultats des tests sans données privées. Une compilation réussie ne remplace pas un essai matériel.
