//! The launcher's own texts: the English one is the key, the second is the Spanish one (empty: not translated yet, the English
//! shows). The texts of the options window come from `schema::labels` and are added to the list for the translation template.

use std::sync::OnceLock;

const UI: &[(&str, &str)] = &[
    ("Ready.", "Listo."),
    ("Start!", "¡Iniciar!"),
    ("Map", "Mapa"),
    ("Map:", "Mapa:"),
    ("Vehicle", "Vehículo"),
    ("Vehicle:", "Vehículo:"),
    ("Driver", "Conductor"),
    ("Driver:", "Conductor:"),
    ("On foot (no vehicle)", "A pie (sin vehículo)"),
    ("Start place", "Lugar de inicio"),
    ("Start place:", "Lugar de inicio:"),
    ("The map's first place", "El primer lugar del mapa"),
    ("Date and time", "Fecha y hora"),
    ("Use the situation's date and time (from the map)", "Usar la fecha y hora de la situación (del mapa)"),
    ("Use the current date and time", "Usar la fecha y hora actuales"),
    ("Choose date and time...", "Elegir fecha y hora..."),
    ("Select time and date...", "Seleccionar hora y fecha..."),
    ("Time:", "Hora:"),
    ("Date:", "Fecha:"),
    ("Current time", "Hora actual"),
    ("Current date", "Fecha actual"),
    ("Current year", "Año actual"),
    ("Day", "Día"),
    ("Afternoon", "Tarde"),
    ("Night", "Noche"),
    ("Morning", "Mañana"),
    ("Weekday:", "Día de la semana:"),
    ("Holidays:", "Festivos:"),
    ("Monday", "Lunes"),
    ("Tuesday", "Martes"),
    ("Wednesday", "Miércoles"),
    ("Thursday", "Jueves"),
    ("Friday", "Viernes"),
    ("Saturday", "Sábado"),
    ("Sunday", "Domingo"),
    ("If you do not change the date, the game uses the situation's or its own.", "Si no cambias la fecha, el juego usa la de la situación o la suya."),
    ("Accept", "Aceptar"),
    ("Cancel", "Cancelar"),
    ("Close", "Cerrar"),
    ("Browse...", "Examinar..."),
    ("Saved situation", "Situación guardada"),
    ("Free start (vehicle on the map)", "Inicio libre (vehículo en el mapa)"),
    ("Start from:", "Empezar desde:"),
    ("Situation:", "Situación:"),
    ("Only this map's situations", "Solo las situaciones de este mapa"),
    ("Traffic:", "Tráfico:"),
    ("Passengers at the stops", "Pasajeros en las paradas"),
    ("Timetable (scheduled buses)", "Horario (autobuses programados)"),
    ("Put the vehicle into service at the start", "Poner el vehículo en servicio al empezar"),
    ("Options", "Opciones"),
    ("Launcher settings", "Ajustes del launcher"),
    ("Tutorials", "Tutoriales"),
    ("Tutorial {}", "Tutorial {}"),
    ("openOMSI settings", "Ajustes de openOMSI"),
    ("Save", "Guardar"),
    ("Saved.", "Guardado."),
    ("Could not save the settings: {}", "No se pudieron guardar los ajustes: {}"),
    ("Reload", "Recargar"),
    ("Reset to the default", "Restablecer el valor por defecto"),
    ("Changes are saved to {} when you press Save.", "Los cambios se guardan en {} al pulsar Guardar."),
    ("Language", "Idioma"),
    ("Language:", "Idioma:"),
    ("Theme", "Tema"),
    ("Theme:", "Tema:"),
    ("Dark", "Oscuro"),
    ("Light", "Claro"),
    ("Original (OMSI 2 orange)", "Original (naranja de OMSI 2)"),
    ("Close the launcher when the game starts", "Cerrar el launcher al iniciar el juego"),
    ("OMSI 2 folder", "Carpeta de OMSI 2"),
    ("openOMSI program", "Programa openOMSI"),
    ("Not found", "No encontrado"),
    ("Look for updates", "Buscar actualizaciones"),
    ("Checking...", "Comprobando..."),
    ("You have the latest version.", "Tienes la última versión."),
    ("New version {}: {}", "Nueva versión {}: {}"),
    ("Could not check ({})", "No se pudo comprobar ({})"),
    ("Create translation template", "Crear plantilla de traducción"),
    ("Template written to {}", "Plantilla escrita en {}"),
    ("OMSI 2 not found", "No se encontró OMSI 2"),
    ("Tell me where OMSI 2 is installed (the folder with Vehicles and maps).", "Indica la carpeta donde tienes OMSI 2 instalado (la que contiene Vehicles y maps)."),
    ("openOMSI not found", "No se encontró openOMSI"),
    ("Tell me where openomsi.exe is.", "Indica dónde está openomsi.exe."),
    ("Choose the OMSI 2 folder", "Elige la carpeta de OMSI 2"),
    ("That folder has no maps and Vehicles.", "Esa carpeta no tiene maps ni Vehicles."),
    ("Starting openOMSI...", "Iniciando openOMSI..."),
    ("Could not start openOMSI: {}", "No se pudo iniciar openOMSI: {}"),
    ("Show the command", "Mostrar el comando"),
    ("Hide the launcher", "Ocultar el launcher"),
    ("Other", "Otros"),
    ("This setting is shown only for some graphics modes.", "Este ajuste solo se muestra con algunos modos gráficos."),
    ("OMSI", "OMSI"),
    ("Off", "Desactivado"),
    ("Default", "Por defecto"),
    ("Automatic", "Automático"),
    ("Original", "Original"),
    ("Graphics", "Gráficos"),
    ("Driving", "Conducción"),
    ("Camera", "Cámara"),
    ("Sound", "Sonido"),
    ("Gameplay", "Juego"),
    ("General", "General"),
    ("Display", "Pantalla"),
    ("Volume", "Volumen"),
    ("Passengers", "Pasajeros"),
    ("Simulation", "Simulación"),
    ("Interface", "Interfaz"),
    ("Navigator", "Navegador"),
    ("Keyboard & mouse", "Teclado y ratón"),
    ("Game controllers", "Mandos"),
    ("Driver's view", "Vista del conductor"),
    ("Outside views", "Vistas exteriores"),
    ("Fullscreen", "Pantalla completa"),
    ("V-sync", "Sincronización vertical"),
    ("Clouds", "Nubes"),
    ("Doppler effect", "Efecto Doppler"),
    ("Collisions with vehicles", "Colisiones con vehículos"),
    ("Collisions with objects (walls, poles)", "Colisiones con objetos (muros, postes)"),
    ("Collisions with people", "Colisiones con personas"),
    ("Start at the real time", "Empezar a la hora real"),
    ("Start on today's date", "Empezar en la fecha de hoy"),
    ("Window size", "Tamaño de la ventana"),
    ("View distance", "Distancia de visión"),
    ("Anti-aliasing", "Antialiasing"),
    ("Mirrors", "Espejos"),
    ("Maintenance", "Mantenimiento"),
    ("Traffic", "Tráfico"),
    ("Random traffic", "Tráfico aleatorio"),
    ("Boarding", "Subida de pasajeros"),
    ("Game interface size", "Tamaño de la interfaz del juego"),
    ("Interface opacity", "Opacidad de la interfaz"),
    ("Frame rate in the corner", "Fotogramas por segundo en la esquina"),
    ("Frame limit", "Límite de fotogramas"),
    ("Driving keys", "Teclas de conducción"),
    ("Disable plugins", "Desactivar plugins"),
];

pub fn all() -> &'static [(&'static str, &'static str)] {
    static ALL: OnceLock<Vec<(&'static str, &'static str)>> = OnceLock::new();
    ALL.get_or_init(|| {
        let mut v: Vec<(&'static str, &'static str)> = UI.to_vec();
        v.extend(crate::texts_es::ES.iter().copied());
        v.extend(crate::texts_es2::ES2.iter().copied());
        for l in crate::schema::labels() {
            if !v.iter().any(|(k, _)| *k == l) {
                v.push((l, ""));
            }
        }
        v
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every row of the options window has a Spanish text (the option values may stay as they are: numbers, brand names).
    #[test]
    fn every_option_row_is_translated_into_spanish() {
        let es: std::collections::HashSet<&str> = all().iter().filter(|(_, e)| !e.is_empty()).map(|(k, _)| *k).collect();
        let mut missing = vec![];
        for t in crate::schema::tabs() {
            if !es.contains(t.label) {
                missing.push(t.label);
            }
            for c in t.cols {
                if !es.contains(c.title) {
                    missing.push(c.title);
                }
                for r in c.rows {
                    if matches!(r.kind, crate::schema::Kind::Custom(_)) {
                        continue;
                    }
                    if !es.contains(r.label) {
                        missing.push(r.label);
                    }
                }
            }
        }
        missing.sort_unstable();
        missing.dedup();
        assert!(missing.is_empty(), "no Spanish text for: {missing:?}");
    }
}
