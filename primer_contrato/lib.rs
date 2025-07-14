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
        pub fn agregar_usuario_sistema(&mut self, nombre: String, apellido: String, direccion: String, email: String, rol: Rol) -> Result <(), String>{
            let account_id = self.env().caller();
            self.priv_agregar_usuario_sistema(account_id, nombre, apellido, direccion, email, rol)
        }
        fn priv_agregar_usuario_sistema(&mut self, account_id: AccountId, nombre: String, apellido: String, direccion: String, email: String, rol: Rol) -> Result <(), String>{
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
            self.priv_cargar_producto(account_id, nombre, descripcion, precio, categoria, stock)
        }
        fn priv_cargar_producto(&mut self, account_id: AccountId, nombre: String, descripcion: String, precio: u32, categoria: String, stock: u32) -> Result<(), String>{
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
            self.priv_crear_publicacion(account_id, productos_a_publicar)
        }
        fn priv_crear_publicacion(&mut self, account_id: AccountId, productos_a_publicar: Vec<(u32, u32)>) -> Result<(), String> {
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
            self.priv_visualizar_productos_de_publicacion(id_publicacion)
        }
        fn priv_visualizar_productos_de_publicacion(&self, id_publicacion: u32) -> Result<Publicacion, String> {
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
            self.priv_modificar_rol(account_id, nuevo_rol)
        }
        fn priv_modificar_rol(&mut self, account_id: AccountId, nuevo_rol: Rol) -> Result<(), String>{
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
            self.priv_crear_orden_de_compra(account_id, id_publicacion)
        }
        fn priv_crear_orden_de_compra(&mut self, account_id: AccountId, id_publicacion: u32) -> Result<(), String>{
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
            self.priv_enviar_compra(account_id, id_orden)
        }
        fn priv_enviar_compra(&mut self, account_id: AccountId, id_orden: u32) -> Result<(), String>{
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
            self.priv_recibir_compra(account_id, id_orden)
        }
        fn priv_recibir_compra(&mut self, account_id: AccountId, id_orden: u32) -> Result<(), String>{ 
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
            if self.ordenes_de_compra.contains(&id_orden){
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
            if self.publicaciones.contains(&id_publicacion){
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

    #[derive(Clone, Debug)]
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

    #[derive(Clone, Debug)]
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

    #[derive(Clone, PartialEq, Debug)]
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
        use ink::env::test;
        use ink::primitives::AccountId;

        fn default_accounts() -> test::DefaultAccounts<ink::env::DefaultEnvironment> {
            test::default_accounts::<ink::env::DefaultEnvironment>()
        }

        #[ink::test]
        fn test_agregar_usuario_comprador() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            let resultado = contrato.agregar_usuario_sistema(
                "Alice".to_string(),
                "Apellido".to_string(),
                "Direccion".to_string(),
                "alice@mail.com".to_string(),
                Rol::Comp,
            );
            assert!(resultado.is_ok(), "No se pudo agregar usuario Comprador");

            let usuario = contrato.usuarios.get(accounts.alice).expect("Usuario no encontrado");
            assert_eq!(usuario.rol, Rol::Comp);
            assert!(usuario.datos_comprador.is_some());
            assert!(usuario.datos_vendedor.is_none());
        }

        #[ink::test]
        fn test_agregar_usuario_vendedor() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            let resultado = contrato.agregar_usuario_sistema(
                "Bob".to_string(),
                "Apellido".to_string(),
                "Direccion".to_string(),
                "bob@mail.com".to_string(),
                Rol::Vend,
            );
            assert!(resultado.is_ok(), "No se pudo agregar usuario Vendedor");

            let usuario = contrato.usuarios.get(accounts.bob).expect("Usuario no encontrado");
            assert_eq!(usuario.rol, Rol::Vend);
            assert!(usuario.datos_comprador.is_none());
            assert!(usuario.datos_vendedor.is_some());
        }

        #[ink::test]
        fn test_agregar_usuario_con_rol_ambos() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
            let resultado = contrato.agregar_usuario_sistema(
                "Charlie".to_string(),
                "Apellido".to_string(),
                "Direccion".to_string(),
                "charlie@mail.com".to_string(),
                Rol::Ambos,
            );
            assert!(resultado.is_ok(), "No se pudo agregar usuario con rol Ambos");

            let usuario = contrato.usuarios.get(accounts.charlie).expect("Usuario no encontrado");
            assert_eq!(usuario.rol, Rol::Ambos);
            assert!(usuario.datos_comprador.is_some());
            assert!(usuario.datos_vendedor.is_some());
        }

        #[ink::test]
        fn test_agregar_usuario_ya_existente_falla() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            test::set_caller::<ink::env::DefaultEnvironment>(accounts.django);
            let _ = contrato.agregar_usuario_sistema(
                "Django".to_string(),
                "Apellido".to_string(),
                "Direccion".to_string(),
                "django@mail.com".to_string(),
                Rol::Comp,
            );

            // Reintento con el mismo caller
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.django);
            let resultado = contrato.agregar_usuario_sistema(
                "Django2".to_string(),
                "Otro".to_string(),
                "OtraDir".to_string(),
                "otro@mail.com".to_string(),
                Rol::Vend,
            );

            assert!(resultado.is_err(), "El sistema debería rechazar registros duplicados");
            assert_eq!(resultado.unwrap_err(), "El usuario ya esta registrado.");
        }

        // ===========================
        // TESTS: cargar_producto
        // ===========================

        #[ink::test]
        fn test_cargar_producto_con_usuario_vendedor() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            let _ = contrato.agregar_usuario_sistema(
                "Vendedor".to_string(),
                "Apellido".to_string(),
                "Direccion".to_string(),
                "vend@mail.com".to_string(),
                Rol::Vend,
            );

            let result = contrato.cargar_producto(
                "Producto 1".to_string(),
                "Descripción".to_string(),
                100,
                "Categoria".to_string(),
                50,
            );

            assert!(result.is_ok(), "El producto debería cargarse correctamente");

            let producto_insertado = contrato.historial_productos.get(1);
            assert!(producto_insertado.is_some(), "No se encontró el producto cargado");

            let (producto, stock) = producto_insertado.unwrap();
            assert_eq!(producto.nombre, "Producto 1");
            assert_eq!(producto.precio, 100);
            assert_eq!(stock, 50);
        }

        #[ink::test]
        fn test_cargar_producto_con_usuario_con_rol_ambos() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            let _ = contrato.agregar_usuario_sistema(
                "Ambos".to_string(),
                "Apellido".to_string(),
                "Direccion".to_string(),
                "ambos@mail.com".to_string(),
                Rol::Ambos,
            );

            let result = contrato.cargar_producto(
                "Producto A".to_string(),
                "Descripción".to_string(),
                200,
                "Categoria".to_string(),
                25,
            );

            assert!(result.is_ok(), "El producto debería cargarse con rol Ambos");
        }

        #[ink::test]
        fn test_cargar_producto_con_usuario_comprador_falla() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
            let _ = contrato.agregar_usuario_sistema(
                "Charlie".to_string(),
                "Apellido".to_string(),
                "Direccion".to_string(),
                "charlie@mail.com".to_string(),
                Rol::Comp,
            );

            let result = contrato.cargar_producto(
                "Producto X".to_string(),
                "Descripción".to_string(),
                150,
                "Categoria".to_string(),
                10,
            );

            assert!(result.is_err(), "Un comprador no debería poder cargar productos");
            assert_eq!(
                result.unwrap_err(),
                "El usuario no tiene permisos para cargar productos. No es vendedor."
            );
        }

        #[ink::test]
        fn test_cargar_producto_sin_registrarse_falla() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            test::set_caller::<ink::env::DefaultEnvironment>(accounts.eve);

            let result = contrato.cargar_producto(
                "Producto Fantasma".to_string(),
                "Sin dueño".to_string(),
                999,
                "Categoria".to_string(),
                1,
            );

            assert!(result.is_err(), "Usuario no registrado no debería poder cargar producto");
            assert_eq!(result.unwrap_err(), "No existe el usuario.");
        }

        #[ink::test]
        fn test_crear_publicacion_con_rol_vendedor() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            // Registrar usuario con rol Vend
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contrato.agregar_usuario_sistema(
                "Vendedor".to_string(),
                "Apellido".to_string(),
                "Dirección".to_string(),
                "vend@mail.com".to_string(),
                Rol::Vend,
            ).unwrap();

            // Cargar un producto
            contrato.cargar_producto(
                "Producto 1".to_string(),
                "Descripción".to_string(),
                100,
                "categoria".to_string(),
                10,
            ).unwrap();

            // Crear publicación
            let resultado = contrato.crear_publicacion(vec![(1, 2)]);
            assert!(resultado.is_ok(), "El vendedor debería poder crear publicación");

            // Validar que se haya guardado
            let publicacion = contrato.historial_publicaciones.get(0).expect("No se encontró publicación");
            assert_eq!(publicacion.0, 0); // ID de la publicación
            assert_eq!(publicacion.1.precio_final, 200); // 100 * 2
        }

        #[ink::test]
        fn test_crear_publicacion_con_rol_ambos() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            // Registrar usuario con rol Ambos
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            contrato.agregar_usuario_sistema(
                "Ambos".to_string(),
                "Apellido".to_string(),
                "Dirección".to_string(),
                "ambos@mail.com".to_string(),
                Rol::Ambos,
            ).unwrap();

            contrato.cargar_producto(
                "Producto A".to_string(),
                "Desc".to_string(),
                50,
                "algo".to_string(),
                5,
            ).unwrap();

            let resultado = contrato.crear_publicacion(vec![(1, 1)]);
            assert!(resultado.is_ok(), "El usuario con rol Ambos debería poder publicar");

            let publicacion = contrato.historial_publicaciones.get(0).expect("No se encontró publicación");
            assert_eq!(publicacion.1.precio_final, 50);
        }

        #[ink::test]
        fn test_crear_publicacion_con_rol_comprador_falla() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
            contrato.agregar_usuario_sistema(
                "Comprador".to_string(),
                "Apellido".to_string(),
                "Dirección".to_string(),
                "comprador@mail.com".to_string(),
                Rol::Comp,
            ).unwrap();

            // Forzar producto en sistema (no debería importar)
            contrato.dimension_logica_productos = 1;
            contrato.historial_productos.insert(1, &(
                Producto {
                    id: 1,
                    nombre: "X".to_string(),
                    descripcion: "Y".to_string(),
                    precio: 999,
                    categoria: "algo".to_string(),
                },
                10
            ));

            let resultado = contrato.crear_publicacion(vec![(1, 1)]);
            assert!(resultado.is_err(), "Comprador no debería poder publicar");
            assert_eq!(resultado.unwrap_err(), "El usuario no es vendedor.");
        }

        #[ink::test]
        fn test_crear_publicacion_usuario_no_existente_falla() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            test::set_caller::<ink::env::DefaultEnvironment>(accounts.eve);
            let resultado = contrato.crear_publicacion(vec![(1, 1)]);
            assert!(resultado.is_err(), "Usuario no registrado no debería poder publicar");
            assert_eq!(resultado.unwrap_err(), "No existe el usuario.");
        }

        #[ink::test]
        fn test_crear_publicacion_con_producto_inexistente_falla() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            // Usuario válido
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.frank);
            contrato.agregar_usuario_sistema(
                "Frank".to_string(),
                "Apellido".to_string(),
                "Dirección".to_string(),
                "frank@mail.com".to_string(),
                Rol::Vend,
            ).unwrap();

            // Intenta publicar producto inexistente
            let resultado = contrato.crear_publicacion(vec![(42, 1)]); // producto ID 42 no existe
            assert!(resultado.is_err(), "Debería fallar por producto no existente");
            assert_eq!(
                resultado.unwrap_err(),
                "Uno de los productos a calcular no se encuentra cargado."
            );
        }

        #[ink::test]
        fn test_visualizar_publicacion_existente() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            // Registrar usuario vendedor
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contrato.agregar_usuario_sistema(
                "Vendedor".to_string(),
                "Apellido".to_string(),
                "Direccion".to_string(),
                "vend@mail.com".to_string(),
                Rol::Vend,
            ).unwrap();

            // Cargar producto
            contrato.cargar_producto(
                "Producto test".to_string(),
                "Descripción".to_string(),
                100,
                "categoria".to_string(),
                10,
            ).unwrap();

            // Crear publicación
            contrato.crear_publicacion(vec![(1, 2)]).unwrap();

            // Visualizar publicación
            let resultado = contrato.visualizar_productos_de_publicacion(0);
            assert!(resultado.is_ok(), "Debería encontrar la publicación con ID 0");

            let publicacion = resultado.unwrap();
            assert_eq!(publicacion.id, 0);
            assert_eq!(publicacion.precio_final, 200);
            assert_eq!(publicacion.productos.len(), 1);
            assert_eq!(publicacion.productos[0], (1, 2));
        }

        #[ink::test]
        fn test_visualizar_publicacion_inexistente() {
            let mut contrato = PrimerContrato::default();

            let resultado = contrato.visualizar_productos_de_publicacion(999); // No existe
            assert!(resultado.is_err(), "No debería encontrar la publicación inexistente");

            assert_eq!(
                resultado.unwrap_err(),
                "No se encontro la publicacion.".to_string()
            );
        }
        #[ink::test]
        fn test_modificar_rol_comprador_a_ambos() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            // Usuario comienza como Comprador
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contrato.agregar_usuario_sistema(
                "Alice".to_string(),
                "Apellido".to_string(),
                "Direccion".to_string(),
                "alice@mail.com".to_string(),
                Rol::Comp,
            ).unwrap();

            // Modifica rol a Ambos
            let result = contrato.modificar_rol(Rol::Ambos);
            assert!(result.is_ok(), "El cambio de rol a Ambos debería funcionar");

            // Validar que tiene ambos datos
            let usuario = contrato.usuarios.get(accounts.alice).unwrap();
            assert_eq!(usuario.rol, Rol::Ambos);
            assert!(usuario.datos_comprador.is_some());
            assert!(usuario.datos_vendedor.is_some());
        }

        #[ink::test]
        fn test_modificar_rol_vendedor_a_ambos() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            contrato.agregar_usuario_sistema(
                "Bob".to_string(),
                "Apellido".to_string(),
                "Direccion".to_string(),
                "bob@mail.com".to_string(),
                Rol::Vend,
            ).unwrap();

            let result = contrato.modificar_rol(Rol::Ambos);
            assert!(result.is_ok(), "El cambio de rol a Ambos debería funcionar");

            let usuario = contrato.usuarios.get(accounts.bob).unwrap();
            assert_eq!(usuario.rol, Rol::Ambos);
            assert!(usuario.datos_vendedor.is_some());
            assert!(usuario.datos_comprador.is_some());
        }

        #[ink::test]
        fn test_modificar_rol_a_rol_actual_falla() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
            contrato.agregar_usuario_sistema(
                "Charlie".to_string(),
                "Apellido".to_string(),
                "Direccion".to_string(),
                "charlie@mail.com".to_string(),
                Rol::Comp,
            ).unwrap();

            let result = contrato.modificar_rol(Rol::Comp);
            assert!(result.is_err(), "No debería poder cambiar al mismo rol");
            assert_eq!(result.unwrap_err(), "El usuario ya posee ese rol.");
        }

        #[ink::test]
        fn test_modificar_rol_usuario_no_existente_falla() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            test::set_caller::<ink::env::DefaultEnvironment>(accounts.eve);
            let result = contrato.modificar_rol(Rol::Vend);

            assert!(result.is_err(), "Usuario no registrado no debería poder cambiar rol");
            assert_eq!(result.unwrap_err(), "No existe el usuario.");
        }

        #[ink::test]
        fn test_crear_orden_con_usuario_comprador() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            // Vendedor publica
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contrato.agregar_usuario_sistema("Vendedor".into(), "A".into(), "Dir".into(), "v@mail.com".into(), Rol::Vend).unwrap();
            contrato.cargar_producto("P1".into(), "Desc".into(), 100, "cat".into(), 5).unwrap();
            contrato.crear_publicacion(vec![(1, 2)]).unwrap();

            // Comprador compra
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            contrato.agregar_usuario_sistema("Comprador".into(), "B".into(), "Dir".into(), "c@mail.com".into(), Rol::Comp).unwrap();
            let result = contrato.crear_orden_de_compra(0);

            assert!(result.is_ok(), "El comprador debería poder crear orden de compra");

            // Validar que la orden fue creada
            let orden = contrato.historial_ordenes_de_compra.get(0).expect("No se creó la orden");
            assert_eq!(orden.0, 0);
            assert_eq!(orden.1.estado, EstadoCompra::Pendiente);
            assert_eq!(orden.1.info_publicacion.2, 200); // precio total
        }

        #[ink::test]
        fn test_crear_orden_con_usuario_ambos() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            // Vendedor
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contrato.agregar_usuario_sistema("Vendedor".into(), "A".into(), "Dir".into(), "v@mail.com".into(), Rol::Vend).unwrap();
            contrato.cargar_producto("P2".into(), "Desc".into(), 50, "cat".into(), 3).unwrap();
            contrato.crear_publicacion(vec![(1, 1)]).unwrap();

            // Comprador con rol Ambos
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
            contrato.agregar_usuario_sistema("Charlie".into(), "C".into(), "Dir".into(), "charlie@mail.com".into(), Rol::Ambos).unwrap();

            let result = contrato.crear_orden_de_compra(0);
            assert!(result.is_ok(), "Usuario con rol Ambos debería poder comprar");
        }

        #[ink::test]
        fn test_crear_orden_con_usuario_vendedor_falla() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            // Publicación del vendedor
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contrato.agregar_usuario_sistema("Vendedor".into(), "A".into(), "Dir".into(), "v@mail.com".into(), Rol::Vend).unwrap();
            contrato.cargar_producto("P3".into(), "Desc".into(), 90, "cat".into(), 5).unwrap();
            contrato.crear_publicacion(vec![(1, 1)]).unwrap();

            // Intenta comprar con el mismo vendedor
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            let result = contrato.crear_orden_de_compra(0);

            assert!(result.is_err(), "Un vendedor no puede comprar");
            assert_eq!(result.unwrap_err(), "El usuario no esta autorizado para realizar una compra. ERROR: No posee el rol comprador.");
        }

        #[ink::test]
        fn test_crear_orden_usuario_no_registrado_falla() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            // Vendedor publica
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contrato.agregar_usuario_sistema("Vendedor".into(), "A".into(), "Dir".into(), "v@mail.com".into(), Rol::Vend).unwrap();
            contrato.cargar_producto("P4".into(), "Desc".into(), 100, "cat".into(), 1).unwrap();
            contrato.crear_publicacion(vec![(1, 1)]).unwrap();

            // Usuario no registrado
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.eve);
            let result = contrato.crear_orden_de_compra(0);

            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), "No existe el usuario.");
        }

        #[ink::test]
        fn test_crear_orden_con_stock_insuficiente_falla() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            // Vendedor
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contrato.agregar_usuario_sistema("V".into(), "A".into(), "Dir".into(), "v@mail.com".into(), Rol::Vend).unwrap();
            contrato.cargar_producto("P5".into(), "Desc".into(), 100, "cat".into(), 1).unwrap();
            contrato.crear_publicacion(vec![(1, 2)]).unwrap(); // Stock insuficiente

            // Comprador
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            contrato.agregar_usuario_sistema("C".into(), "B".into(), "Dir".into(), "c@mail.com".into(), Rol::Comp).unwrap();

            let result = contrato.crear_orden_de_compra(0);
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), "No hay stock suficiente.");
        }

        #[ink::test]
        fn test_crear_orden_con_publicacion_inexistente_falla() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            // Usuario comprador válido
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            contrato.agregar_usuario_sistema("Comprador".into(), "B".into(), "Dir".into(), "c@mail.com".into(), Rol::Comp).unwrap();

            let result = contrato.crear_orden_de_compra(999); // ID inexistente
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), "No se encontro la publicacion.");
        }

        #[ink::test]
        fn test_enviar_compra_con_vendedor_correcto() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            // Vendedor crea publicacion
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contrato.agregar_usuario_sistema("V".into(), "X".into(), "Dir".into(), "v@mail.com".into(), Rol::Vend).unwrap();
            contrato.cargar_producto("Prod".into(), "Desc".into(), 100, "cat".into(), 10).unwrap();
            contrato.crear_publicacion(vec![(1, 1)]).unwrap();

            // Comprador crea orden
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            contrato.agregar_usuario_sistema("C".into(), "Y".into(), "Dir".into(), "c@mail.com".into(), Rol::Comp).unwrap();
            contrato.crear_orden_de_compra(0).unwrap();

            // Vendedor envía
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            let result = contrato.enviar_compra(0);
            assert!(result.is_ok(), "El vendedor debería poder enviar la compra");

            let (_, orden) = contrato.historial_ordenes_de_compra.get(0).unwrap();
            assert_eq!(orden.estado, EstadoCompra::Enviado);
        }

        #[ink::test]
        fn test_enviar_compra_con_usuario_que_no_es_vendedor() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            // Setup con orden creada por comprador
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contrato.agregar_usuario_sistema("V".into(), "X".into(), "Dir".into(), "v@mail.com".into(), Rol::Vend).unwrap();
            contrato.cargar_producto("Prod".into(), "Desc".into(), 100, "cat".into(), 10).unwrap();
            contrato.crear_publicacion(vec![(1, 1)]).unwrap();

            test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            contrato.agregar_usuario_sistema("C".into(), "Y".into(), "Dir".into(), "c@mail.com".into(), Rol::Comp).unwrap();
            contrato.crear_orden_de_compra(0).unwrap();

            // Comprador intenta enviar
            let result = contrato.enviar_compra(0);
            assert_eq!(result.unwrap_err(), "El usuario no posee el rol de vendedor.");
        }

        #[ink::test]
        fn test_enviar_compra_con_vendedor_que_no_es_dueño_de_publicacion() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            // Vendedor 1 crea publicacion
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contrato.agregar_usuario_sistema("V1".into(), "X".into(), "Dir".into(), "v1@mail.com".into(), Rol::Vend).unwrap();
            contrato.cargar_producto("Prod".into(), "Desc".into(), 100, "cat".into(), 10).unwrap();
            contrato.crear_publicacion(vec![(1, 1)]).unwrap();

            // Comprador crea orden
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
            contrato.agregar_usuario_sistema("C".into(), "Y".into(), "Dir".into(), "c@mail.com".into(), Rol::Comp).unwrap();
            contrato.crear_orden_de_compra(0).unwrap();

            // Vendedor 2 intenta enviar
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            contrato.agregar_usuario_sistema("V2".into(), "Z".into(), "Dir".into(), "v2@mail.com".into(), Rol::Vend).unwrap();
            let result = contrato.enviar_compra(0);
            assert_eq!(result.unwrap_err(), "La publicacion buscada no pertenece a este vendedor.");
        }

        #[ink::test]
        fn test_enviar_compra_orden_inexistente() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contrato.agregar_usuario_sistema("V".into(), "X".into(), "Dir".into(), "v@mail.com".into(), Rol::Vend).unwrap();

            let result = contrato.enviar_compra(99);
            assert_eq!(result.unwrap_err(), "No existe la orden buscada.");
        }

        #[ink::test]
        fn test_enviar_compra_usuario_no_registrado() {
            let mut contrato = PrimerContrato::default();

            // Nadie registrado
            let result = contrato.enviar_compra(0);
            assert_eq!(result.unwrap_err(), "No existe el usuario.");
        }

        #[ink::test]
        fn test_recibir_compra_con_estado_enviado() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            // Vendedor crea publicación
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contrato.agregar_usuario_sistema("V".into(), "X".into(), "Dir".into(), "v@mail.com".into(), Rol::Vend).unwrap();
            contrato.cargar_producto("Prod".into(), "Desc".into(), 100, "cat".into(), 5).unwrap();
            contrato.crear_publicacion(vec![(1, 1)]).unwrap();

            // Comprador crea orden
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            contrato.agregar_usuario_sistema("C".into(), "Y".into(), "Dir".into(), "c@mail.com".into(), Rol::Comp).unwrap();
            contrato.crear_orden_de_compra(0).unwrap();

            // Vendedor envía
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contrato.enviar_compra(0).unwrap();

            // Comprador recibe
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            let result = contrato.recibir_compra(0);
            assert!(result.is_ok(), "El comprador debería poder recibir la orden");

            let (_, orden) = contrato.historial_ordenes_de_compra.get(0).unwrap();
            assert_eq!(orden.estado, EstadoCompra::Recibido);
        }

        #[ink::test]
        fn test_recibir_compra_estado_pendiente_falla() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            // Vendedor publica
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contrato.agregar_usuario_sistema("V".into(), "X".into(), "Dir".into(), "v@mail.com".into(), Rol::Vend).unwrap();
            contrato.cargar_producto("P".into(), "D".into(), 100, "cat".into(), 3).unwrap();
            contrato.crear_publicacion(vec![(1, 1)]).unwrap();

            // Comprador ordena
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            contrato.agregar_usuario_sistema("C".into(), "Y".into(), "Dir".into(), "c@mail.com".into(), Rol::Comp).unwrap();
            contrato.crear_orden_de_compra(0).unwrap();

            // Recibe sin que se haya enviado
            let result = contrato.recibir_compra(0);
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), "El producto todavia no fue enviado.");
        }

        #[ink::test]
        fn test_recibir_compra_con_usuario_vendedor_falla() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            // Vendedor crea y publica
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contrato.agregar_usuario_sistema("V".into(), "X".into(), "Dir".into(), "v@mail.com".into(), Rol::Vend).unwrap();
            contrato.cargar_producto("P".into(), "D".into(), 100, "cat".into(), 5).unwrap();
            contrato.crear_publicacion(vec![(1, 1)]).unwrap();

            // Comprador ordena y vendedor envía
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            contrato.agregar_usuario_sistema("C".into(), "Y".into(), "Dir".into(), "c@mail.com".into(), Rol::Comp).unwrap();
            contrato.crear_orden_de_compra(0).unwrap();

            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contrato.enviar_compra(0).unwrap();

            // Vendedor intenta recibir (no puede)
            let result = contrato.recibir_compra(0);
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), "El usuario no posee el rol de comprador.");
        }

        #[ink::test]
        fn test_recibir_compra_orden_inexistente_falla() {
            let accounts = default_accounts();
            let mut contrato = PrimerContrato::default();

            test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            contrato.agregar_usuario_sistema("C".into(), "Y".into(), "Dir".into(), "c@mail.com".into(), Rol::Comp).unwrap();

            let result = contrato.recibir_compra(42);
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), "No existe la orden buscada.");
        }

        #[ink::test]
        fn test_recibir_compra_usuario_no_registrado_falla() {
            let mut contrato = PrimerContrato::default();

            // Nadie registrado
            let result = contrato.recibir_compra(0);
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), "No existe el usuario.");
        }
    }
}
