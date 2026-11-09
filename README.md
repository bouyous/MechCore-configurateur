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
- Mode expert avec un modèle Ollama local, pour rechercher des fonctions existantes dans les sources MeshCore et consulter les extraits qui étayent la réponse.
- Interface en 43 langues, avec choix au premier démarrage et modification possible dans le menu **Langue**.

## Installer et lancer sous Windows

L’exécutable pour Windows 64 bits (x64) est disponible dans la section [Télécharger](#télécharger) en bas de cette page. Lancez-le par double-clic, choisissez votre langue, puis continuez vers le configurateur. Le choix est enregistré dans `%LOCALAPPDATA%\MeshStudio\language.json`. Le bouton **Langue** dans la barre latérale permet de le changer. Les profils et compilations sont enregistrés dans `%LOCALAPPDATA%\MeshStudio`.

Les catalogues intégrés couvrent les 24 langues officielles de l’UE et les langues officielles nationales des autres pays européens, dont le catalan d’Andorre, le monténégrin et le romanche de Suisse. La sélection et l’interface fonctionnent hors ligne. Les consignes de flash et d’effacement ont fait l’objet d’une relecture ciblée. Les termes anglais entre parenthèses éclairent les actions sensibles et correspondent aux noms des rôles dans la documentation MeshCore. Les journaux des outils externes et les textes déjà enregistrés dans les profils gardent leur langue d’origine.

Pour télécharger les sources et compiler ou flasher un firmware, installez [Git](https://git-scm.com/install/windows) et [PlatformIO Core](https://docs.platformio.org/en/latest/core/installation/index.html). Ajoutez `git` et `pio` au PATH, ou renseignez le chemin complet de PlatformIO dans « Compiler et flasher ». Les pilotes USB dépendent de la carte. Une connexion Internet est nécessaire pour les téléchargements.

Pour compiler le configurateur depuis le code source, installez Rust avec Cargo et les outils de compilation C++ Windows.

Dans le dossier contenant `Cargo.toml` :

```powershell
cargo build --release --locked
cargo run --release --locked
```

L’exécutable est produit dans `target/release/mesh-studio.exe`.

## Préparer un firmware

1. Dans « Configurateur », actualiser les versions, choisir la version et le type de firmware, puis télécharger les sources. Un dossier MeshCore local peut aussi être importé.
2. Choisir la carte et sa cible, puis régler les options souhaitées.
3. Si des périphériques sont utilisés, vérifier le schéma constructeur et les broches. Appliquer les fonctions intégrées souhaitées ; confirmer le câblage lorsqu’il est demandé.
4. Dans « Compiler et flasher », vérifier le récapitulatif et lancer la compilation. Les fichiers `.bin`, `.uf2` ou `.hex` disponibles et le reçu `build.json` sont copiés dans `%LOCALAPPDATA%\MeshStudio\builds\<identifiant>\firmware`.
5. Sauvegarder les données de la carte, vérifier que la compilation lui correspond, indiquer le port série et confirmer avant de flasher. L’option d’effacement supprime les données de l’appareil.

Un profil JSON décrit une configuration : ce n’est pas un firmware à flasher.
Les profils échangés ne choisissent ni l’exécutable PlatformIO, ni le dossier source, ni le correctif, ni le port de cette installation. Sélectionnez les sources à nouveau après l’import.

PlatformIO exécute les scripts des sources, correctifs et dépendances avec les droits de votre compte Windows. La copie des sources protège les originaux des modifications faites par le configurateur. Compilez uniquement des sources et correctifs de confiance, sans lancer l’application en administrateur.

### USB, Bluetooth et Wi-Fi

Le bouton de flash utilise un port série USB de type `COM3`. Les choix Bluetooth et Wi-Fi servent à compiler un firmware Compagnon avec l’interface correspondante, si la cible la propose. Le configurateur ne détecte pas d’appareils sans fil et ne transfère aucun firmware par Bluetooth ou Wi-Fi.

Pour une carte et un firmware compatibles OTA, suivez les procédures MeshCore pour le [transfert Wi-Fi sur ESP32](https://github.com/meshcore-dev/MeshCore/blob/main/docs/faq.md#72-q-how-to-update-esp32-based-devices-over-the-air) et le [DFU Bluetooth sur nRF52](https://github.com/meshcore-dev/MeshCore/blob/main/docs/faq.md#71-q-how-to-update-nrf-rak-t114-seed-xiao-repeater-and-room-server-firmware-over-the-air-using-the-new-simpler-dfu-app). Les formats de fichier et prérequis varient selon la carte.

## Mode expert

1. Installez une version récente d’[Ollama pour Windows](https://docs.ollama.com/windows). Dans PowerShell, téléchargez le modèle proposé par défaut avec `ollama pull qwen2.5-coder:3b`. Vous pouvez choisir un autre modèle de texte déjà installé.
2. [Désactivez les fonctions cloud d’Ollama](https://docs.ollama.com/faq#how-do-i-disable-ollamas-cloud-features) et redémarrez-le. Par exemple, quittez Ollama depuis la zone de notification, puis lancez ces commandes dans une fenêtre PowerShell et laissez-la ouverte pendant la recherche.

   ```powershell
   $env:OLLAMA_NO_CLOUD = '1'
   ollama serve
   ```

3. Chargez les sources MeshCore dans **Configurateur**, choisissez votre cible, puis ouvrez **Mode expert**. Essayez « Où se trouve la mise à jour OTA du répéteur ESP32 ? ». La réponse s’accompagne de passages identifiés par fichier et numéro de ligne.

Le configurateur contacte uniquement le serveur Ollama de cet ordinateur sur `127.0.0.1:11434` et vérifie le mode local avant d’envoyer du contenu. La recherche est en lecture seule. Elle couvre les principaux dossiers de code, d’exemples, de variantes et de documentation. Le modèle peut manquer un passage ou se tromper sur la compatibilité d’une carte. Vérifiez les extraits avant d’appliquer un correctif ou d’activer une option.

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
- `src/expert.rs` : recherche dans les sources et synthèse par Ollama local.
- `locales/translations.json` : libellés et messages traduits, intégrés à l’exécutable.
- `src/atelier_bot.h` : bot embarqué facultatif.
- `tests/` : contrôle natif C++ et ses substituts matériels ; les tests Rust se trouvent aussi dans les modules `src/`.

## Mettre le projet sur GitHub

Utiliser le contenu de ce dossier `app/` comme racine du dépôt : inclure `src/`, `tests/`, `Cargo.toml`, `Cargo.lock`, `README.md`, `HARDWARE.md` et `.gitignore`. Le fichier `.gitignore` exclut les données personnelles, les sources téléchargées et les binaires générés. Vérifier manuellement les fichiers ajoutés, surtout les exports de profils et leurs chemins locaux.

Le configurateur est distribué sous [licence MIT](LICENSE). Les sources MeshCore téléchargées conservent leurs propres licences. Toute redistribution de firmware doit en respecter les conditions. [MeshCore](https://github.com/meshcore-dev/MeshCore) est le projet amont. LoRa est une marque de Semtech.

Pour contribuer, décrire le problème ou la modification, préciser la carte et la version concernées, et joindre les résultats des tests sans données privées. Une compilation réussie ne remplace pas un essai matériel.

## Télécharger

**[Télécharger Mesh Studio v0.3.1 pour Windows x64 (.exe)](https://github.com/bouyous/MechCore-configurateur/releases/download/v0.3.1/Mesh-Studio-v0.3.1-windows-x64.exe)**

Version expérimentale. L’exécutable Windows est dépourvu de signature Authenticode. La somme SHA-256 est disponible sur la [page de la version v0.3.1](https://github.com/bouyous/MechCore-configurateur/releases/tag/v0.3.1).
