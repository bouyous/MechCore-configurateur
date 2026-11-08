# Matériel et câblage

Le menu GPIO & UART propose un catalogue recherchable de familles de périphériques utiles à un relais LoRa. Aucun capteur n’est obligatoire. Les contacts numériques, les niveaux de sortie au démarrage et le bot BME280 facultatif peuvent être appliqués aux fonctions intégrées du répéteur ESP32. Les autres périphériques disposent d’un plan de câblage sauvegardé/exporté dans le profil ; ils nécessitent un pilote et ne sont pas activés par ce plan seul.

La sélection d’une carte analyse ses fichiers de variante puis consulte les mêmes fichiers dans le dépôt officiel MeshCore sur Internet, au tag sélectionné. Avec des sources locales, la consultation utilise `main` et l’indique. Une panne réseau conserve l’analyse locale et affiche une erreur. Aucun document téléchargé n’est exécuté.

Les lignes affichées sont celles du processeur identifié (indices Arduino sur nRF52 avec NUM_DIGITAL_PINS fourni). Une ligne candidate n’est pas une garantie qu’elle est exposée sur un connecteur. Les réservations des macros, y compris les alias simples et les variantes conditionnelles, sont regroupées par prudence. Les expressions C complexes, le schéma, les fonctions alternatives RP2040, le partage des bus et les révisions de carte nécessitent une vérification constructeur. Les processeurs inconnus ne reçoivent pas de brochage supposé. Avant compilation, les GPIO et le BME280 actifs sont aussi contrôlés contre l’analyse locale.

Documentation consultée le 6 novembre 2026 :

- Variantes MeshCore : https://github.com/meshcore-dev/MeshCore/tree/main/variants
- GPIO ESP32 : https://docs.espressif.com/projects/esp-idf/en/v5.0.5/esp32/api-reference/peripherals/gpio.html
- GPIO ESP32-S3 : https://docs.espressif.com/projects/esp-idf/en/stable/esp32s3/api-reference/peripherals/gpio.html
- UART Arduino-ESP32 : https://docs.espressif.com/projects/arduino-esp32/en/latest/api/serial.html
- BME280 : https://www.bosch-sensortec.com/products/environmental-sensors/humidity-sensors-bme280/
- Identité LoRa, marque de Semtech : https://www.semtech.com/company/brand-resources

Les liens propres à chaque périphérique sont également accessibles dans son menu. Le bandeau utilise un mot-symbole LoRa vectoriel natif ; aucune image distante n’est nécessaire au démarrage.
