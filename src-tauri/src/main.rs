// Évite d'ouvrir une console en plus de la fenêtre sous Windows (version finale)
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    ariane_lib::run()
}
