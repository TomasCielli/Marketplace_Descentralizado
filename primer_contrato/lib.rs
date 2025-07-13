//COMPILA

/////////////////////////// IMPORTANTE ///////////////////////////
    /* 
            PERSISTENCIA DE DATOS

                1) Los datos que persistirán son solo aquellos bajo la etiqueta #[ink(storage)].
                2) Los datos simples solo persitirán si son variables del struct con la etiqueta #[ink(Storage)]
                3) Solo las estructuras StorageVec y Mapping persistirán su información. 
                4) Si se modifica un tipo de dato Vec o HashMap, se debe sobreescribir el respectivo Mapping o StorageVec.
                    Ej: Modifico Usuario, debo reinsertarlo en el Mapping del Sistema. 

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
        dimension_logica_productos: u32, //cantidad de productos, primero suma despues usa la id
    }
    impl PrimerContrato {

        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                usuarios: Mapping::default(),
                historial_publicaciones: StorageVec::new(),
                historial_productos: Mapping::default(),
                historial_ordenes_de_compra:  StorageVec::new(),
                dimension_logica_productos: 0,
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
        pub fn cargar_producto(&mut self, nombre: String, descripcion: String, precio: u32, categoria: String, stock: u32) -> Result<(), String>{
            let account_id = self.env().caller();
            if let Some(usuario) = self.usuarios.get(account_id){
                if (usuario.rol == Rol::Vend) | (usuario.rol == Rol::Ambos){
                    self.dimension_logica_productos = self.dimension_logica_productos.checked_add(1).ok_or("Error al sumar.")?;
                    self.historial_productos.insert(self.dimension_logica_productos, &(Producto::cargar_producto(self.dimension_logica_productos, nombre, descripcion, precio, categoria), stock));
                    Ok(())
                }
                else {
                    Err("El usuario no tiene permisos para cargar productos. No es vendedor.".to_string())
                }
            }
            else {
                Err("No existe el usuario.".to_string())
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
                                return Err("El producto todavia no fue enviado.".to_string());
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
    impl Producto{
        fn cargar_producto(id: u32, nombre: String, descripcion: String, precio: u32, categoria: String) -> Producto{
            let categoria_limpia = categoria.to_lowercase().chars().filter(|c| c.is_ascii_alphabetic()).collect();
            Producto{
                id,
                nombre,
                descripcion,
                precio,
                categoria: categoria_limpia,
            }
        }
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
        calificaciones: (bool, bool), //(Calificacion Vendedor, Calificacion Comprador)
    }
    impl OrdenCompra{
        
        fn crear_orden_de_compra(id_orden: u32, publicacion: Publicacion, id_comprador: AccountId) -> OrdenCompra{
            let id_publicacion = publicacion.id; //Para mejor legibilidad
            let productos = publicacion.productos;
            let precio_final = publicacion.precio_final;
            let id_vendedor = publicacion.id_vendedor;
            let info_publicacion = (id_publicacion, productos, precio_final, id_vendedor);
            let calificaciones = (false, false);

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
}