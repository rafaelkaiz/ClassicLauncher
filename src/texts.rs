//! The launcher's own texts: the English one is the key, the second is the Spanish one (empty: not translated yet, the English
//! shows). The texts of the options window come from `schema::labels` and are added to the list for the translation template.

use std::sync::OnceLock;

const UI: &[(&str, &str)] = &[
    ("Ready.", "Listo."),
    ("Start!", "¡Iniciar!"),
    ("Driver:", "Conductor:"),
    ("Date and time", "Fecha y hora"),
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
    ("Accept", "Aceptar"),
    ("Cancel", "Cancelar"),
    ("Close", "Cerrar"),
    ("Browse...", "Examinar..."),
    ("Traffic:", "Tráfico:"),
    ("Passengers at the stops", "Pasajeros en las paradas"),
    ("Timetable (scheduled buses)", "Horario (autobuses programados)"),
    ("Options", "Opciones"),
    ("Launcher settings", "Ajustes del launcher"),
    ("Tutorials", "Tutoriales"),
    ("Tutorial {}", "Tutorial {}"),
    ("Save", "Guardar"),
    ("Could not save the settings: {}", "No se pudieron guardar los ajustes: {}"),
    ("Theme", "Tema"),
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
    ("Create translation template", "Crear plantilla de traducción"),
    ("OMSI 2 not found", "No se encontró OMSI 2"),
    ("Tell me where OMSI 2 is installed (the folder with Vehicles and maps).", "Indica la carpeta donde tienes OMSI 2 instalado (la que contiene Vehicles y maps)."),
    ("openOMSI not found", "No se encontró openOMSI"),
    ("Tell me where openomsi.exe is.", "Indica dónde está openomsi.exe."),
    ("Choose the OMSI 2 folder", "Elige la carpeta de OMSI 2"),
    ("That folder has no maps and Vehicles.", "Esa carpeta no tiene maps ni Vehicles."),
    ("Starting openOMSI...", "Iniciando openOMSI..."),
    ("Could not start openOMSI: {}", "No se pudo iniciar openOMSI: {}"),
    ("Show the command", "Mostrar el comando"),
    ("Driver's view", "Vista del conductor"),
];

pub fn all() -> &'static [(&'static str, &'static str)] {
    static ALL: OnceLock<Vec<(&'static str, &'static str)>> = OnceLock::new();
    ALL.get_or_init(|| {
        let mut v: Vec<(&'static str, &'static str)> = UI.to_vec();
        v.extend(crate::texts_es::ES.iter().copied());
        v.extend(crate::texts_es2::ES2.iter().copied());
        v
    })
}
