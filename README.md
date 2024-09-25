# Backup Application - Group 39

## Overview
This project is a Rust application designed to facilitate backups when the screen is not accessible. 
The user can initiate a backup to an external drive (e.g., USB stick) using a conventional (activation + confirmation) pattern command. 

We have two possible options (to be chosen in the configuration file):
- drawing a clockwise rectangle, starting from the top left side; 
in this case a second clockwise rectangle must be drawn to confirm the backup, 
a counter-clockwise rectangle will cancel the backup 
- use a combination of buttons `ctrl + alt + b` pressed for 5 seconds and then three consecutive mouse clicks (`left` to confirm, `right` to cancel).

After that, the backup will start on the specified path in the configuration file.

## Instructions
For further information on how to use the application, please refer to the attached documentation.

## Contibutors
This project is part of the Programmazione di Sistema course at the Politecnico di Torino and was designed and developed by: 
- [Raffaele Pane - S305485](https://github.com/bred91)
- [Veronica Mattei - S310707](https://github.com/veronicamattei)
- [Jacopo Spaccatrosi - S285891](https://github.com/Jack313131)
