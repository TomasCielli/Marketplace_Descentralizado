//COMPILA

/////////////////////////// IMPORTANTE ///////////////////////////
    /* 
            ACTUALIZACION / TOMAS /

                1) Nombre de los roles -> Ambos, Vend, Comp //Para evitar warnings
                2) Correccion de todos los warnings
                3) Cambios en comprobar_rol //Ahora maneja los ids de vendedor y comprador directamente
                4) Limitar rango de calificacion (1..5)
                5) Limitar la cantidad de calificaciones por compra (una para cada uno). Cambios en calificar, CHEKEAR!


            PERSISTENCIA DE DATOS

                1) Los datos que persistirán son solo aquellos bajo la etiqueta #[ink(storage)].
                2) Los datos simples solo persitirán si son variables del struct con la etiqueta #[ink(Storage)]
                3) Solo las estructuras StorageVec y Mapping persistirán su información. 
                4) Si se modifica un tipo de dato Vec o HashMap, se debe sobreescribir el respectivo Mapping o StorageVec.
                    Ej: Modifico Usuario, debo reinsertarlo en el Mapping del Sistema. 

            DATOS SOBRE MAPPING Y STORAGEVEC

                1) El .get() devuelve una copia del elemento, no se pierde la propiedad. Si se quiere modificar la estructura se debe pisar el valor anterior.

            CONSULTAS:
            
                1) Los productos deben estar previamente cargados en sistema? 
                2) Puede haber productos (con o sin stock) que no esten en publicaciones? //repreguntar
                3) Que es la categoria del producto?: Enum o String
                4) LINEA 75. Hay que sobreescribir los datos del mapping de usuario (este fue modificado)?
                5) LINEA 219. Amerita error?
                6) La cancelacion devuelve stock a su valor anterior?
                7) Cargo test no funciona
    */

#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(non_local_definitions)]

#[ink::contract]
mod primer_contrato {
    use ink::storage::Mapping;
    use ink::storage::StorageVec;
    use ink::prelude::vec::Vec;
    use ink::prelude::string::{String, ToString};

/////////////////////////// SISTEMA ///////////////////////////
    #[ink(storage)]
    pub struct PrimerContrato {
        usuarios: Mapping<AccountId, Usuario>, //Se persiste la información de los usuarios. 
        historial_publicaciones: StorageVec<(u32, Publicacion)>, //(id, publicacion) //Se persisten las publicaciones realizadas.
        historial_productos: Mapping<u32, (Producto, u32)>, // <id, (producto, stock)> //Se persisten los productos de mis sistema.
        historial_ordenes_de_compra: StorageVec<(u32, OrdenCompra)>, //(id, orden)
        value: bool, //Si borras esto se rompe todo. En serio, fijate. (Hay que sacarle las fn, algún día se hará).  
    }
    impl PrimerContrato {

        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                usuarios: Mapping::default(),
                historial_publicaciones: StorageVec::new(),
                historial_productos: Mapping::default(),
                historial_ordenes_de_compra:  StorageVec::new(),
                value: false,                //lo dejamos xq explota
            }
        }

        #[ink(message)]
        pub fn agregar_usuario_sitema(&mut self, nombre: String, apellido: String, direccion: String, email: String, rol: Rol) -> Result <(), String>{
            let account_id = self.env().caller();    // el caller obtiene la cuenta del usuario que esta invocando la fn
            if self.usuarios.get(account_id).is_some(){
                Err("El usuario ya esta registrado.".to_string())
            } else {
                let usuario = Usuario::nuevo(account_id, nombre, apellido, direccion, email, rol);  // generamos el usuario
                self.usuarios.insert(account_id, &usuario); // lo metemos en el  hashmap/mapping de usuarios 
                Ok(())
            }
        }

        #[ink(message)]
        pub fn crear_publicacion(&mut self, productos_a_publicar: Vec<(u32, u32)>) -> Result<(), String> {
            let account_id = self.env().caller();
            if let Some(mut usuario) = self.usuarios.get(account_id){
                let id_publicacion = self.historial_publicaciones.len();
                let precio_final = self.calcular_precio_final(productos_a_publicar.clone())?;
                let publicacion = usuario.crear_publicacion(productos_a_publicar, precio_final, id_publicacion, account_id)?;
                self.historial_publicaciones.push(&(id_publicacion, publicacion));
                self.usuarios.insert(account_id, &usuario); //lo sobreescribe
                Ok(())
            }
            else {
                Err("No existe el usuario.".to_string())
            }

        }
        
        #[ink(message)]
        pub fn visualizar_productos_de_publicacion(&self, id_publicacion: u32) -> Result<Publicacion, String> {
            for i in 0..self.historial_publicaciones.len() { // desde 0 a dimF (la longitud del vec)
                if let Some((id, publicacion)) = self.historial_publicaciones.get(i) { //si hay un elemento cargado en la posicion
                    if id == id_publicacion {  //revisa que tenga el mismo id
                        return Ok(publicacion.clone()); //devuelve la publicacion
                    }
                }
            }
            Err("No se encontro la publicacion.".to_string()) //sino, devuelve error
        }

        #[ink(message)]
        pub fn modificar_rol(&mut self, nuevo_rol: Rol) -> Result<(), String>{
            let account_id = self.env().caller();
            if let Some(mut usuario) = self.usuarios.get(account_id){
                usuario.modificar_rol(nuevo_rol)?;
                self.usuarios.insert(account_id, &usuario);
                Ok(())
            }
            else {
                Err("No existe el usuario.".to_string())
            }
        }
        
        #[ink(message)]
        pub fn crear_orden_de_compra(&mut self, id_publicacion: u32) -> Result<(), String>{
            let account_id = self.env().caller();
            if let Some(mut usuario) = self.usuarios.get(account_id){
                let publicacion = self.visualizar_productos_de_publicacion(id_publicacion)?;
                self.hay_stock_suficiente(publicacion.productos.clone())?; //si hay stock devuelve error, termina la funcion. Sino sigue ejecutando
                let id_orden = self.historial_ordenes_de_compra.len(); //guarda la id
                let orden_de_compra = usuario.crear_orden_de_compra(id_orden, publicacion.clone(), account_id)?; // <---- Que revise el stock primero
                self.descontar_stock(publicacion.productos)?; //descuenta el stock del producto
                self.historial_ordenes_de_compra.push(&(id_orden, orden_de_compra));
                self.usuarios.insert(account_id, &usuario); //sobreescribe el usuario en el mapping
                Ok(())  
            }
            else {
                Err("No existe el usuario.".to_string())
            }
        }
        
        #[ink(message)]
        pub fn enviar_compra(&mut self, id_orden: u32) -> Result<(), String>{
            let account_id = self.env().caller();
            if let Some(usuario) = self.usuarios.get(account_id){
                for i in 0..self.historial_ordenes_de_compra.len() {
                    if let Some((id, mut orden_de_compra)) = self.historial_ordenes_de_compra.get(i){
                        if id == id_orden {
                            let id_publicacion = orden_de_compra.info_publicacion.0;
                            usuario.enviar_compra(id_publicacion)?;
                            orden_de_compra.estado = EstadoCompra::Enviado;
                            let _ = self.historial_ordenes_de_compra.set(i, &(id, orden_de_compra));
                            return Ok(());
                        }
                    }
                }
                Err("No existe la orden buscada.".to_string())
            }
            else {
                Err("No existe el usuario.".to_string())
            }
        }
        
        #[ink(message)]
        pub fn recibir_compra(&mut self, id_orden: u32) -> Result<(), String>{
            let account_id = self.env().caller(); 
            if let Some(usuario) = self.usuarios.get(account_id){ //busca el usuario
                for i in 0..self.historial_ordenes_de_compra.len() { //recorre las ordenes
                    if let Some((id, mut orden_de_compra)) = self.historial_ordenes_de_compra.get(i){
                        if id == id_orden { //si encuentra la orden
                            if orden_de_compra.estado != EstadoCompra::Enviado{
                                return Err("El producto todavia ni fue enviado master, espera unos dias maquinola".to_string());
                            }
                            usuario.recibir_compra(id_orden)?; //continua sino, error
                            orden_de_compra.estado = EstadoCompra::Recibido;
                            let _ = self.historial_ordenes_de_compra.set(i, &(id, orden_de_compra));
                            return Ok(())
                        }
                    }
                }
                Err("No existe la orden buscada.".to_string()) //si no encontro la orden devuelve error
            }
            else {
                Err("No existe el usuario.".to_string()) //si no encontro el usuario devuelve error
            }
        }

        #[ink(message)]
        pub fn cancelar_compra(&mut self, id_orden:u32) -> Result<(), String>{ // <------- FALTA TERMINAR
            let account_id = self.env().caller(); 
            if let Some(usuario) = self.usuarios.get(account_id){ //busca el usuario
                for i in 0..self.historial_ordenes_de_compra.len(){
                    if let Some((id, mut orden_de_compra)) = self.historial_ordenes_de_compra.get(i){
                        if id == id_orden{
                            let publicacion = orden_de_compra.info_publicacion.clone();
                            if orden_de_compra.estado != EstadoCompra::Pendiente{
                                return Err("La compra no puede ser cancelada. No esta en estado pendiente.".to_string())
                            }
                            else {
                                let id_vendedor = orden_de_compra.info_publicacion.3;
                                let id_comprador = orden_de_compra.id_comprador;
                                let rol = usuario.comprobar_rol(id_vendedor, id_comprador)?;
                                if rol == Rol::Comp{
                                    orden_de_compra.cancelacion.1 = true;
                                }
                                else if orden_de_compra.cancelacion.1{
                                    orden_de_compra.cancelacion.0 = true;
                                    orden_de_compra.estado = EstadoCompra::Cancelada;
                                    self.aumentar_stock(publicacion.1)?;  //PREGUNTAR 
                                }
                                else {
                                    return Err("El comprador no desea cancelar la compra.".to_string())
                                }
                                let _ = self.historial_ordenes_de_compra.set(i, &(id, orden_de_compra));
                                return Ok(())
                            }
                        }
                    }
                }
                Err("No se encontro la orden de compra.".to_string())
            }
            else {
                Err("No existe el usuario.".to_string()) //si no encontro el usuario devuelve error
            }
        }

        //Version vieja
        /*#[ink(message)]
        pub fn calificar(&mut self, id_orden:u32, calificacion: u8) -> Result<(), String>{
            if (calificacion < 1) & (calificacion > 5){
                return Err("El valor de la calificacion no es valido (1..5).".to_string())
            }
            let account_id = self.env().caller(); 
            if let Some(usuario) = self.usuarios.get(account_id){
                for i in 0..self.historial_ordenes_de_compra.len(){
                    if let Some((id, orden_de_compra)) = self.historial_ordenes_de_compra.get(i){
                        if id_orden == id{
                            let id_comprador = orden_de_compra.id_comprador;
                            let id_vendedor = orden_de_compra.info_publicacion.3;
                            let rol = usuario.comprobar_rol(id_vendedor, id_comprador)?;
                            if rol == Rol::Comp{
                                if 
                                let id_vendedor = orden_de_compra.info_publicacion.3;
                                self.puntuar_vendedor(id_vendedor, calificacion)?;
                                return Ok(())
                            }
                            else { //es el vendedor
                                let id_comprador = orden_de_compra.id_comprador;
                                self.puntuar_comprador(id_comprador, calificacion)?;
                                return Ok(())
                            }

                        }
                    }    
                }
                Err("No se encontro una orden de compra con esa id.".to_string())
            }
            else {
                Err("El usuario no existe".to_string())
            }
        }*/

        #[ink(message)]
        pub fn calificar(&mut self, id_orden:u32, calificacion: u8) -> Result<(), String>{
            if (calificacion < 1) & (calificacion > 5){ //Revisa que la calificacion este en rango
                return Err("El valor de la calificacion no es valido (1..5).".to_string())
            }
            let account_id = self.env().caller(); 
            if let Some(usuario) = self.usuarios.get(account_id){ //Busca que el usuario exita
                for i in 0..self.historial_ordenes_de_compra.len(){ //Recorre el vector de ordenes de compra
                    if let Some((id, mut orden_de_compra)) = self.historial_ordenes_de_compra.get(i){
                        if id_orden == id{ //Si encuentra una orden con la id deseada
                            let id_comprador = orden_de_compra.id_comprador; //guarda la id del comprador y del vendedor
                            let id_vendedor = orden_de_compra.info_publicacion.3;
                            let rol = usuario.comprobar_rol(id_vendedor, id_comprador)?; //revisa que rol cumple el usuario en esta compra (comprador o vendedor)

                            if rol == Rol::Comp{ //si es el comprador
                                if orden_de_compra.calificaciones.1.is_some(){ //si el comprador ya califico
                                    return Err("El comprador ya califico al vendedor.".to_string()) //devuelve error
                                }
                                else { //sino califico
                                    orden_de_compra.calificaciones.1 = Some(calificacion); //modifica la calificacion de la orden (None -> Some(calificacion))
                                    self.puntuar_vendedor(id_vendedor, calificacion)?; //carga la calificacion en el vendedor
                                }
                            }
                            else { //sino, es el vendedor
                                if orden_de_compra.calificaciones.0.is_some(){ //si el vendedor ya califico
                                    return Err("El vendedor ya califico al comprador.".to_string()) //devuelve error
                                }
                                else{ //sino califico
                                    orden_de_compra.calificaciones.0 = Some(calificacion); //modifica la calificacion de la orden (None -> Some(calificacion))
                                    self.puntuar_comprador(id_comprador, calificacion)?; //carga la calificacion en el comprador
                                }
                            }
                            self.historial_ordenes_de_compra.set(i, &(id, orden_de_compra)); //sobreescribe el vector de ordenes, con la nueva orden modificada
                            return Ok(()) //devuelve Ok
                        }
                    }    
                }
                Err("No se encontro una orden de compra con esa id.".to_string())
            }
            else {
                Err("El usuario no existe".to_string())
            }
        }

        #[ink(message)]
        pub fn mostrar_puntuacion_vendedor(&self) -> Result<u8, String>{
            let account_id = self.env().caller(); 
            if let Some(usuario) = self.usuarios.get(account_id){
                usuario.mostrar_puntuacion_vendedor()
            }
            else {
                Err("El usuario no se encuentra cargado en sistema.".to_string())
            }
        }

        #[ink(message)]
        pub fn mostrar_puntuacion_comprador(&self) -> Result<u8, String>{
            let account_id = self.env().caller(); 
            if let Some(usuario) = self.usuarios.get(account_id){
                usuario.mostrar_puntuacion_comprador()
            }
            else {
                Err("El usuario no se encuentra cargado en sistema.".to_string())
            }
        }

        fn puntuar_vendedor(&mut self, id_vendedor: AccountId, calificacion: u8) -> Result<(), String>{
            if let Some(mut vendedor) = self.usuarios.get(id_vendedor){
                if let Some(ref mut datos_vendedor) = vendedor.datos_vendedor{
                    datos_vendedor.reputacion_como_vendedor.push(calificacion);
                    self.usuarios.insert(id_vendedor, &vendedor);
                    Ok(())
                }
                else{
                    Err("El vendedor no tiene datos cargados.".to_string())
                }
            }
            else{
                Err("No existe un vendedor con ese id".to_string())
            }
        }

        fn puntuar_comprador(&mut self, id_comprador: AccountId, calificacion: u8) -> Result<(), String>{
            if let Some(mut comprador) = self.usuarios.get(id_comprador){
                if let Some(ref mut datos_comprador) = comprador.datos_comprador{
                    datos_comprador.reputacion_como_comprador.push(calificacion);
                    self.usuarios.insert(id_comprador, &comprador);
                    Ok(())
                }
                else{
                    Err("El comprador no tiene datos cargados.".to_string())
                }
            }
            else{
                Err("No existe un comprador con ese id".to_string())
            }
        }

        fn calcular_precio_final(&self, productos_publicados: Vec<(u32, u32)>) -> Result<u32, String>{
            let mut total: u32 = 0;
            for (id, cantidad) in productos_publicados{
                if let Some((producto, _stock)) = self.historial_productos.get(id){
                    total = total.checked_add(producto.precio.checked_mul(cantidad)
                    .ok_or("Overflow al multiplicar precio por cantidad.")?)
                    .ok_or("Overflow al acumular el total.")?;
                }
                else {
                    return Err("Uno de los productos a calcular no se encuentra cargado.".to_string())
                }
            }
            Ok(total)
        }

      
        fn hay_stock_suficiente(&self, productos_cantidades: Vec<(u32, u32)>) -> Result<(), String>{
            for (id, cantidad) in productos_cantidades{ //recorremos el vector de productos y cantidades
                if let Some((_producto, stock)) = self.historial_productos.get(id){ //si encuentra el producto
                    if stock < cantidad{ //y el stock es menor
                        return Err("No hay stock suficiente.".to_string()) //devuelve error
                    }
                }
                else{
                    return Err("No se encontro el producto.".to_string()) //si no encuentra el producto en el mapping tambien devuelve error
                }
            }
            Ok(()) //si no sale por ninguna de las opciones anterios esta todo Ok, devuelve Ok
        }
        
        
        fn descontar_stock(&mut self, productos_cantidades: Vec<(u32, u32)>) -> Result<(), String>{
            for (id, cantidad) in productos_cantidades{
                if let Some ((producto, mut stock)) = self.historial_productos.get(id){
                    stock = stock.checked_sub(cantidad).ok_or("Error al restar stock")?; //siempre va a haber stock minimo sufiente porque se revisa antes con la fn "hay_stock_suficiente"
                    self.historial_productos.insert(id, &(producto, stock)); //sobreescribe el vector
                }
                else {
                    return Err("No se encontro el producto.".to_string())
                }
            }
            Ok(())
        }

        fn aumentar_stock(&mut self, productos_cantidades: Vec<(u32, u32)>) -> Result<(), String>{
            for (id, cantidad) in productos_cantidades{
                if let Some ((producto, mut stock)) = self.historial_productos.get(id){
                    stock = stock.checked_add(cantidad).ok_or("Error al sumar stock")?;
                    self.historial_productos.insert(id, &(producto, stock)); //sobreescribe el vector
                }
                else {
                    return Err("No se encontro el producto.".to_string())
                }
            }
            Ok(())
        }
    }
    impl Default for PrimerContrato {
        fn default() -> Self {
            Self::new()
        }
    }
       

/////////////////////////// USUARIO ///////////////////////////

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    struct Usuario{
        id_usuario: AccountId,
        nombre: String,
        apellido: String,
        direccion: String,
        email: String,
        rol: Rol,
        datos_comprador: Option<Comprador>,
        datos_vendedor: Option<Vendedor>,
    }
    impl Usuario {
        
        pub fn nuevo(id: AccountId,nombre: String,apellido: String,direccion: String,email: String,rol: Rol) -> Usuario {
            Usuario {
                id_usuario: id,
                nombre,
                apellido,
                direccion,
                email,
                rol: rol.clone(),
                datos_comprador: match rol {
                    Rol::Comp | Rol::Ambos => Some(Comprador {         // si el rol es comprador o ambos se inicializan 
                        ordenes_de_compra: Vec::new(),
                        reputacion_como_comprador: Vec::new(),
                    }),
                    _ => None,
                },
                datos_vendedor: match rol {
                    Rol::Vend | Rol::Ambos => Some(Vendedor {            // si el rol es Vendedor o ambos se inicializan
                        productos: Vec::new(),
                        publicaciones: Vec::new(),
                        reputacion_como_vendedor: Vec::new(),
                    }),
                    _ => None,
                },
            }
        }

        fn crear_publicacion(&mut self, productos_a_publicar: Vec<(u32, u32)>, precio_final: u32, id_publicacion: u32, id_vendedor: AccountId) -> Result<Publicacion, String>{  //productos_a_publicar = Vec<(id, cantidad)>
            if self.rol == Rol::Comp {
                Err("El usuario no es vendedor.".to_string())
            }
            else if let Some(ref mut datos_vendedor) = self.datos_vendedor {
                Ok(datos_vendedor.crear_publicacion(productos_a_publicar, precio_final, id_publicacion, id_vendedor))
            } 
            else {
                Err("El vendedor no tiene datos.".to_string())
            }
        }

        fn modificar_rol(&mut self, nuevo_rol: Rol) -> Result<(), String>{
            if nuevo_rol == self.rol {
                Err("El usuario ya posee ese rol.".to_string())
            }
            else {
                self.rol = nuevo_rol.clone(); // cargo el nuevo rol
                match nuevo_rol{
                    Rol::Comp => { //si el nuevo rol es comprador
                        if self.datos_comprador.is_none(){ //y no tiene ningun dato anterior de comprador
                            let ordenes_de_compra = Vec::new();
                            let reputacion_como_comprador = Vec::new();
                            self.datos_comprador = Some(Comprador{ //inicializa todos los datos del comprador
                                ordenes_de_compra,
                                reputacion_como_comprador,
                            });
                        }
                    }

                    Rol::Vend => { //si el nuevo rol es vendedor
                        if self.datos_vendedor.is_none(){ //y no tiene ningun dato anterior de vendedor
                            let productos = Vec::new();
                            let publicaciones = Vec::new();
                            let reputacion_como_vendedor = Vec::new();
                            self.datos_vendedor = Some(Vendedor{ //inicializa todos los datos del vendedor
                                productos,
                                publicaciones,
                                reputacion_como_vendedor,
                            });
                        }
                    }

                    Rol::Ambos => { //lo mismo para ambos
                        if self.datos_comprador.is_none(){
                            let ordenes_de_compra = Vec::new();
                            let reputacion_como_comprador = Vec::new();
                            self.datos_comprador = Some(Comprador{
                                ordenes_de_compra,
                                reputacion_como_comprador,
                            });
                        }
                        if self.datos_vendedor.is_none(){
                            let productos = Vec::new();
                            let publicaciones = Vec::new();
                            let reputacion_como_vendedor = Vec::new();
                            self.datos_vendedor = Some(Vendedor{
                                productos,
                                publicaciones,
                                reputacion_como_vendedor,
                            });
                        }
                    }
                }
                Ok(())
            }
        }
        
        fn crear_orden_de_compra(&mut self, id_publicacion: u32, publicacion: Publicacion, id_comprador: AccountId) -> Result<OrdenCompra, String>{
            if self.rol == Rol::Vend{
                Err("El usuario no esta autorizado para realizar una compra. ERROR: No posee el rol comprador.".to_string())
            }
            else if let Some(ref mut datos_comprador) = self.datos_comprador{
                Ok(datos_comprador.crear_orden_de_compra(id_publicacion, publicacion, id_comprador))
            }
            else{
                Err("No hay datos de comprador.".to_string())
            }
        }

        fn enviar_compra(&self, id_publicacion: u32) -> Result<(), String>{
            if self.rol == Rol::Comp{
                Err("El usuario no posee el rol de vendedor.".to_string())
            }
            else if let Some(ref datos_vendedor) = self.datos_vendedor{
                datos_vendedor.enviar_compra(id_publicacion)
            }
            else {
                Err("El vendedor no tiene datos.".to_string())
            }
        }

        fn recibir_compra(&self, id_orden: u32) -> Result<(), String>{
            if self.rol == Rol::Vend{
                Err("El usuario no posee el rol de comprador.".to_string())
            }
            else if let Some(ref datos_comprador) = self.datos_comprador{
                datos_comprador.recibir_compra(id_orden)
            }
            else {
                Err("No hay datos del comprador.".to_string())
            }
        }

        fn comprobar_rol(&self, id_vendedor: AccountId, id_comprador: AccountId) -> Result<Rol, String>{
            match self.rol{
                Rol::Vend => {
                    if let Some(ref _datos_vendedor) = self.datos_vendedor{
                        if self.es_el_vendedor(id_vendedor){
                            Ok(Rol::Vend)
                        }
                        else {
                            Err("El vendedor no posee esta publicacion.".to_string())
                        }
                    }
                    else {
                        Err("El vendedor no tiene datos cargados.".to_string())
                    }
                },

                Rol::Comp => {
                    if let Some(ref _datos_comprador) = self.datos_comprador{
                        if self.es_el_comprador(id_comprador){
                            Ok(Rol::Comp)
                        }
                        else{
                            Err("El usuario no posee esa orden de compra.".to_string())
                        }
                    }
                    else {
                        Err("El comprador no tiene datos cargados.".to_string())
                    }
                },

                Rol::Ambos => {
                    if self.es_el_vendedor(id_vendedor) {
                        Ok(Rol::Vend)
                    }
                    else if self.es_el_comprador(id_comprador) {
                        Ok(Rol::Comp)
                    }
                    else {
                       Err("El usuario no participa de la compra.".to_string())
                    }
                },
            }
        }

        fn es_el_vendedor(&self, id_vendedor: AccountId) -> bool{
            id_vendedor == self.id_usuario
        }

        fn es_el_comprador(&self, id_comprador: AccountId) -> bool{
            id_comprador == self.id_usuario
        }

        fn mostrar_puntuacion_vendedor(&self) -> Result<u8, String>{
            if self.rol == Rol::Comp{
                Err("El usuario no posee el rol vendedor".to_string())
            }
            else if let Some(ref datos_vendedor) = self.datos_vendedor{
                datos_vendedor.mostrar_puntuacion_vendedor()
            }
            else{
                Err("El vendedor no tiene datos cargados".to_string())
            }
        }

        fn mostrar_puntuacion_comprador(&self) -> Result<u8, String>{
            if self.rol == Rol::Vend{
                Err("El usuario no posee el rol comprador".to_string())
            }
            else if let Some(ref datos_comprador) = self.datos_comprador{
                datos_comprador.mostrar_puntuacion_comprador()
            }
            else{
                Err("El comprador no tiene datos cargados".to_string())
            }
        }
    }



/////////////////////////// COMPRADOR ///////////////////////////

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    struct Comprador{
        ordenes_de_compra: Vec<u32>, // IDs de las ordenes de compra
        reputacion_como_comprador: Vec<u8>, 
    }
    impl Comprador{

        fn crear_orden_de_compra(&mut self, id_publicacion: u32, publicacion: Publicacion, id_comprador: AccountId) -> OrdenCompra{
            self.ordenes_de_compra.push(id_publicacion);
            OrdenCompra::crear_orden_de_compra(id_publicacion, publicacion, id_comprador)
        }

        fn recibir_compra(&self, id_orden: u32) -> Result<(), String>{
            if self.ordenes_de_compra.iter().any(|id| *id == id_orden){
                Ok(())
            }
            else {
                Err("No se encontro la orden de compra.".to_string())
            }
        }

        fn mostrar_puntuacion_comprador(&self) -> Result<u8, String> {
            if self.reputacion_como_comprador.is_empty() {
                return Err("El usuario no tiene puntuaciones cargadas.".to_string());
            }
            let mut total: u8 = self.reputacion_como_comprador.iter().sum();
            let cantidad = self.reputacion_como_comprador.len() as u8;
            total = total.checked_div(cantidad).ok_or("Error al dividir.".to_string())?;
            Ok(total)
        }
    }


/////////////////////////// VENDEDOR ///////////////////////////

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    struct Vendedor{
        productos: Vec<u32>, //IDs de los productos
        publicaciones: Vec<u32>, //IDs de las publicaciones
        reputacion_como_vendedor: Vec<u8>,
    }
    impl Vendedor{
        fn crear_publicacion(&mut self, productos_a_publicar: Vec<(u32, u32)>, precio_final: u32, id_publicacion: u32, id_vendedor: AccountId) -> Publicacion {
            //let precio_total = productos_a_publicar.iter().try_fold(0u64, |acc, producto| acc.checked_add(producto.precio)).ok_or("Overflow en suma de precios")?; 
            let publicacion = Publicacion::crear_publicacion(productos_a_publicar, precio_final, id_publicacion, id_vendedor);
            self.publicaciones.push(id_publicacion);
            publicacion
        }

        fn enviar_compra(&self, id_publicacion: u32) -> Result<(), String>{
            if self.publicaciones.iter().any(|id| *id == id_publicacion){
                Ok(())
            }
            else {
                Err("La publicacion buscada no pertenece a este vendedor.".to_string())
            }
        }

        fn mostrar_puntuacion_vendedor(&self) -> Result<u8, String> {
            if self.reputacion_como_vendedor.is_empty() {
                return Err("El usuario no tiene puntuaciones cargadas.".to_string());
            }
            let mut total: u8 = self.reputacion_como_vendedor.iter().sum();
            let cantidad = self.reputacion_como_vendedor.len() as u8;
            total = total.checked_div(cantidad).ok_or("Error al dividir.".to_string())?;
            Ok(total)
        }
    }



/////////////////////////// ROL ///////////////////////////

    #[derive(Debug, Clone, PartialEq)]    // agrege clone  para comprobar los test 
    #[ink::scale_derive(Encode, Decode, TypeInfo)]                            
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    pub enum Rol {
        Ambos,
        Comp, 
        Vend,
    }




/////////////////////////// PUBLICACION ///////////////////////////

    #[derive(Clone)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    pub struct Publicacion{
        id: u32,
        productos: Vec<(u32, u32)>, // (IDs de los productos, cantidades de ese producto)
        precio_final: u32,
        id_vendedor:AccountId
    }
    impl Publicacion {
        fn crear_publicacion(productos_a_publicar: Vec<(u32, u32)>, precio_final: u32, id_publicacion: u32, id_vendedor: AccountId) -> Publicacion{
            Publicacion{
                id: id_publicacion,
                productos: productos_a_publicar,
                precio_final,
                id_vendedor,
            }
        }
    }


/////////////////////////// PRODUCTO ///////////////////////////

    #[derive(Clone)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    pub struct Producto{
        id: u32,
        nombre: String,
        descripcion: String,
        precio: u32,
        categoria:String,
    }

/////////////////////////// ORDEN DE COMPRA ///////////////////////////

    #[derive(Clone)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    struct OrdenCompra{
        id: u32,
        estado: EstadoCompra,
        cancelacion: (bool, bool),  //(vendedor, comprador) //Para estado cancelada
        info_publicacion: (u32, Vec<(u32, u32)>, u32, AccountId), // (ID de la publicacion, Vec<(IDs de los productos, cantidades de ese producto)>, precio final de la publicacion, ID del Vendedor)
        id_comprador:AccountId,
        calificaciones: (Option<u8>, Option<u8>), //(Calificacion Vendedor, Calificacion Comprador)
    }
    impl OrdenCompra{
        
        fn crear_orden_de_compra(id_orden: u32, publicacion: Publicacion, id_comprador: AccountId) -> OrdenCompra{
            let id_publicacion = publicacion.id; //Para mejor legibilidad
            let productos = publicacion.productos;
            let precio_final = publicacion.precio_final;
            let id_vendedor = publicacion.id_vendedor;
            let info_publicacion = (id_publicacion, productos, precio_final, id_vendedor);
            let calificaciones = (None, None);

            OrdenCompra{
                id: id_orden,
                estado: EstadoCompra::Pendiente,
                cancelacion: (false, false),
                info_publicacion,
                id_comprador, 
                calificaciones,
            }
        }
    }



/////////////////////////// ESTADO DE COMPRA ///////////////////////////

    #[derive(Clone, PartialEq)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    enum EstadoCompra{
        Pendiente,
        Enviado,
        Recibido,
        Cancelada,
    }

/////////////////////////// TESTS ///////////////////////////
    #[cfg(test)]
    mod tests {
        
        use super::*;
       
        fn obtener_usuario_test(contrato: &PrimerContrato, account_id: AccountId) -> Option<Usuario> {
            contrato.usuarios.get(account_id)
        }

        /* ---------------------------------------------------------------   */
        /* -------------------- Test Agregar Usuario ---------------------   */
        /* ---------------------------------------------------------------   */
        #[ink::test]
        fn test_agregar_usuario() {
            let mut contrato = PrimerContrato::default();
            let account_id = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>().alice;    // carteras simuladas ink! define un conjunto de cuentas por defecto que se pueden usar en los tests.
            
            //  Alice es el caller       
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(account_id);

            let resultado = contrato.agregar_usuario_sitema(
                "teo".to_string(),
                "Cortez".to_string(),
                "La Plata".to_string(),
                "teo@mail.com".to_string(),
                Rol::Comp,
            );

            assert!(resultado.is_ok(), "El usuario debería agregarse correctamente");

            let usuario_guardado = obtener_usuario_test(&contrato, account_id);                           // esto esta arriba 
            assert!(usuario_guardado.is_some(), "El usuario debería estar presente en el mapping");

            let usuario = usuario_guardado.unwrap();
            assert_eq!(usuario.nombre, "teo");
            assert_eq!(usuario.rol, Rol::Comp);
        }

        #[ink::test]
        fn test_agregar_usuario_duplicado() {                               // Test que comprueba error 
            let mut contrato = PrimerContrato::default();
            let account_id = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>().bob;

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(account_id);

            let resultado_1 = contrato.agregar_usuario_sitema(                              // se genera un marketPlace default y agrega un usuario
                "teo".to_string(),
                "cortez".to_string(),
                "La Plata".to_string(),
                "teo@mail.com".to_string(),
                Rol::Vend,
            );

            assert!(resultado_1.is_ok(), "Primer registro debería funcionar");              // se genera un usuario y se intenta arreglar


            let resultado_2 = contrato.agregar_usuario_sitema(
                "teo".to_string(),
                "cortez".to_string(),
                "La Plata".to_string(),
                "teo@mail.com".to_string(),
                Rol::Vend,
            );

            assert!(resultado_2.is_err(), "No se debería permitir agregar el mismo usuario dos veces");
            assert_eq!(resultado_2.unwrap_err(), "El usuario ya esta registrado.");
        }

    /* ---------------------------------------------------------------   */
    /* ---------------------------------------------------------------   */
    /* ---------------------------------------------------------------   */
        #[ink::test]
        fn test_crear_publicacion() {
            let mut contrato = PrimerContrato::default();
            let account_id = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>().alice;
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(account_id);

            // Registramos al usuario vendedor
            let _ = contrato.agregar_usuario_sitema(
                "vendedor".to_string(),
                "uno".to_string(),
                "Av 1".to_string(),
                "Messi@gmail.com".to_string(),
                Rol::Vend,
            );

            // Insertamos un producto directamente al historial del sistema (producto id 1)
            let producto = Producto::nuevo(1, "Mouse".to_string(), "Gamer RGB".to_string(), 100, "Computacion".to_string());
            contrato.historial_productos.insert(1, &(producto, 10)); 

            let resultado = contrato.crear_publicacion(vec![(1, 2)]);
            assert!(resultado.is_ok(), "La publicación deberia crearse correctamente");

            // Verifica que se haya guardado en historial_publicaciones
            assert_eq!(contrato.historial_publicaciones.len(), 1);
        }

        #[ink::test]
        fn test_crear_publicacion_usuario_inexistente() {
            let mut contrato = PrimerContrato::default();
            let account_id = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>().alice;
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(account_id);

            let resultado = contrato.crear_publicacion(vec![(1, 2)]);
            assert!(resultado.is_err());
            assert_eq!(resultado.unwrap_err(), "No existe el usuario.");
        }

        #[ink::test]
        fn test_crear_publicacion_producto_inexistente() {
            let mut contrato = PrimerContrato::default();
            let account_id = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>().alice;
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(account_id);

            let _ = contrato.agregar_usuario_sitema(
                "tomas".to_string(),
                "user".to_string(),
                "Argentina".to_string(),
                "tomas@mail.com".to_string(),
                Rol::Vend,
            );

            // Producto con id 99 no existe
            let resultado = contrato.crear_publicacion(vec![(99, 1)]);
            assert!(resultado.is_err());
            assert_eq!(
                resultado.unwrap_err(),
                "Uno de los productos a calcular no se encuentra cargado."
            );
        }
    }

/////////////////////////// INFO ///////////////////////////

    /* ----------------------Importante-O-No---------------------------------*/
    /*Documentacion de ink*/
    /*https://docs.rs/ink_env/4.3.0/ink_env/index.html#modules*/
    // Cosas de los contratos

    /*Development Accounts
    The alice development account will be the authority and sudo account as declared in the genesis state. While at the same time, the following accounts will be pre-funded:

    Alice
    Bob
    Charlie
    Dave
    Eve
    Ferdie


    estas son las cuentas que se deberian permitir en la zona de test  

    compilar o hacer cargo test me genero un monton de quilombos tuve que actualizar varias cosas 
    el comando para probar los test es:= cargo test -- --nocapture


    agrege esto en dependencia
    [patch.crates-io]
    backtrace = "=0.3.74"
    icu_collections = "=1.4.0"
    icu_locale_core = "=1.4.0"
    icu_normalizer = "=1.4.0"
    icu_normalizer_data = "=1.4.0"
    icu_properties = "=1.4.0"
    icu_properties_data = "=1.4.0"
    icu_provider = "=1.4.0"
    idna_adapter = "=1.1.0"
    litemap = "=0.7.0"
    potential_utf = "=0.1.1"
    tinystr = "=0.7.0"
    writeable = "=0.5.0"
    yoke = "=0.7.0"
    zerofrom = "=0.1.5"
    zerotrie = "=0.2.1"
    zerovec = "=0.10.0"

    [toolchain]
    channel = "nightly-2025-07-06"

    version de rustc 1.90.0-nightly
    */

}