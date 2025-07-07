#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(non_local_definitions)]


/* 
    IMPORTANTE

        ARREGLOS URGENTES!!!!

            1) Revisar en TODAS las funciones el uso de .get en general. Consejo: doble click en .get() y ver TODOS los lugares donde se lo llama. Suerte jiji

        
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
            2) Puede haber productos (con o sin stock) que no esten en publicaciones? //preguntada
            3) Que es la categoria del producto?: Enum o String
            4) LINEA 75. Hay que sobreescribir los datos del mapping de usuario (este fue modificado)?
            5) LINEA 219. Amerita error?
            6) La cancelacion devuelve stock a su valor anterior?
}

*/

#[ink::contract]
mod primer_contrato {
    use ink::storage::Mapping;
    use ink::storage::StorageVec;
    use ink::prelude::vec::Vec;
    use ink::prelude::string::{String, ToString};
    
/////////////////////////// ALERTA ALERTA ALERTA EL MAPPING DEVUELVE COPIAS NOMAS, ERGO HAY QUE PISAR TODOS LAS VECES QUE MODIFICAMOS USUARIO /////////////////////////

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
                let publicacion = usuario.crear_publicacion(productos_a_publicar, precio_final, id_publicacion)?;
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
                usuario.modificar_rol(nuevo_rol)
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
                let mut hay_stock = self.hay_stock_suficiente(publicacion.productos.clone())?; //si hay stock devuelve error, termina la funcion. Sino sigue ejecutando
                let id_orden = self.historial_ordenes_de_compra.len(); //guarda la id
                let orden_de_compra = usuario.crear_orden_de_compra(id_orden, publicacion.clone())?; // <---- Que revise el stock primero
                hay_stock = self.descontar_stock(publicacion.productos)?;
                self.historial_ordenes_de_compra.push(&(id_orden, orden_de_compra));
                Ok(())  
            }
            else {
                Err("No existe el usuario.".to_string())
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
                    self.historial_productos.insert(id, &(producto, stock));
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
        
        pub fn nuevo(id: AccountId,nombre: String,apellido: String,direccion: String,email: String,rol: Rol,) -> Usuario {
            Usuario {
                id_usuario: id,
                nombre,
                apellido,
                direccion,
                email,
                rol: rol.clone(),
                datos_comprador: match rol {
                    Rol::COMPRADOR | Rol::AMBOS => Some(Comprador {         // si el rol es comprador o ambos se inicializan 
                        ordenes_de_compra: Vec::new(),
                        reputacion_como_comprador: Vec::new(),
                    }),
                    _ => None,
                },
                datos_vendedor: match rol {
                    Rol::VENDEDOR | Rol::AMBOS => Some(Vendedor {            // si el rol es Vendedor o ambos se inicializan
                        productos: Vec::new(),
                        publicaciones: Vec::new(),
                        reputacion_como_vendedor: Vec::new(),
                    }),
                    _ => None,
                },
            }
        }

        fn crear_publicacion(&mut self, productos_a_publicar: Vec<(u32, u32)>, precio_final: u32, id_publicacion: u32) -> Result<Publicacion, String>{  //productos_a_publicar = Vec<(id, cantidad)>
            if self.rol == Rol::COMPRADOR {
                Err("El usuario no es vendedor.".to_string())
            }
            else if let Some(ref mut datos_vendedor) = self.datos_vendedor {
                Ok(datos_vendedor.crear_publicacion(productos_a_publicar, precio_final, id_publicacion))
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
                    Rol::COMPRADOR => { //si el nuevo rol es comprador
                        if self.datos_comprador.is_none(){ //y no tiene ningun dato anterior de comprador
                            let ordenes_de_compra = Vec::new();
                            let reputacion_como_comprador = Vec::new();
                            self.datos_comprador = Some(Comprador{ //inicializa todos los datos del comprador
                                ordenes_de_compra,
                                reputacion_como_comprador,
                            });
                        }
                    }

                    Rol::VENDEDOR => { //si el nuevo rol es vendedor
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

                    Rol::AMBOS => { //lo mismo para ambos
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
        
        fn crear_orden_de_compra(&mut self, id_publicacion: u32, publicacion: Publicacion) -> Result<OrdenCompra, String>{
            if self.rol == Rol::VENDEDOR{
                Err("El usuario no esta autorizado para realizar una compra. ERROR: No posee el rol comprador.".to_string())
            }
            else if let Some(ref mut datos_comprador) = self.datos_comprador{
                Ok(datos_comprador.crear_orden_de_compra(id_publicacion, publicacion))
            }
            else{
                Err("No hay datos de comprador.".to_string())
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

        fn crear_orden_de_compra(&mut self, id_publicacion: u32, publicacion: Publicacion) -> OrdenCompra{
            self.ordenes_de_compra.push(id_publicacion);
            OrdenCompra::crear_orden_de_compra(id_publicacion, publicacion)
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
        fn crear_publicacion(&mut self, productos_a_publicar: Vec<(u32, u32)>, precio_final: u32, id_publicacion: u32) -> Publicacion {
            //let precio_total = productos_a_publicar.iter().try_fold(0u64, |acc, producto| acc.checked_add(producto.precio)).ok_or("Overflow en suma de precios")?; 
            let publicacion = Publicacion::crear_publicacion(productos_a_publicar, precio_final, id_publicacion);
            self.publicaciones.push(id_publicacion);
            publicacion
        }

    }



/////////////////////////// ROL ///////////////////////////

    #[derive(Clone, PartialEq)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]                            
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    pub enum Rol {
        AMBOS,
        COMPRADOR, 
        VENDEDOR,
    }




/////////////////////////// PUBLICACION ///////////////////////////

    #[derive(Clone)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    pub struct Publicacion{
        id: u32,
        productos: Vec<(u32, u32)>, // (IDs de los productos, cantidades de ese producto)
        precio_final: u32,
    }
    impl Publicacion {
        fn crear_publicacion(productos_a_publicar: Vec<(u32, u32)>, precio_final: u32, id_publicacion: u32) -> Publicacion{
            Publicacion{
                id: id_publicacion,
                productos: productos_a_publicar,
                precio_final,
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

    impl Producto {
        pub fn nuevo(id: u32, nombre: String, descripcion: String, precio: u32, categoria: String,) -> Producto {
            Producto {
                id,
                nombre,
                descripcion,
                precio,
                categoria,
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
        cancelacion: (bool, bool),  //Para estado cancelada
        info_publicacion: (u32, Vec<(u32, u32)>, u32) // (ID de la publicacion, Vec<(IDs de los productos, cantidades de ese producto)>, precio final de la publicacion)
    }
    impl OrdenCompra{
        
        fn crear_orden_de_compra(id_orden: u32, publicacion: Publicacion) -> OrdenCompra{
            let id_publicacion = publicacion.id; //Para mejor legibilidad
            let productos = publicacion.productos;
            let precio_final = publicacion.precio_final;

            let info_publicacion = (id_publicacion, productos, precio_final);

            OrdenCompra{
                id: id_orden,
                estado: EstadoCompra::Pendiente,
                cancelacion: (false, false),
                info_publicacion,
            }
        }
    }



/////////////////////////// ESTADO DE COMPRA ///////////////////////////

    #[derive(Clone)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    enum EstadoCompra{
        Pendiente,
        Enviado,
        Recibido,
        Cancelada,
    }






    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// We test if the default constructor does its job.
        #[ink::test]
        fn default_works() {
            let primer_contrato = PrimerContrato::default();
            assert_eq!(primer_contrato.get(), false);
        }

        /// We test a simple use case of our contract.
        #[ink::test]
        fn it_works() {
            let mut primer_contrato = PrimerContrato::new(false);
            assert_eq!(primer_contrato.get(), false);
            primer_contrato.flip();
            assert_eq!(primer_contrato.get(), true);
        }
    }


    


    /// This is how you'd write end-to-end (E2E) or integration tests for ink! contracts.
    ///
    /// When running these you need to make sure that you:
    /// - Compile the tests with the `e2e-tests` feature flag enabled (`--features e2e-tests`)
    /// - Are running a Substrate node which contains `pallet-contracts` in the background
    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// A helper function used for calling contract messages.
        use ink_e2e::ContractsBackend;

        /// The End-to-End test `Result` type.
        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        /// We test that we can upload and instantiate the contract using its default constructor.
        #[ink_e2e::test]
        async fn default_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            // Given
            let mut constructor = PrimerContratoRef::default();

            // When
            let contract = client
                .instantiate("primer_contrato", &ink_e2e::alice(), &mut constructor)
                .submit()
                .await
                .expect("instantiate failed");
            let call_builder = contract.call_builder::<PrimerContrato>();

            // Then
            let get = call_builder.get();
            let get_result = client.call(&ink_e2e::alice(), &get).dry_run().await?;
            assert!(matches!(get_result.return_value(), false));

            Ok(())
        }

        /// We test that we can read and write a value from the on-chain contract.
        #[ink_e2e::test]
        async fn it_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            // Given
            let mut constructor = PrimerContratoRef::new(false);
            let contract = client
                .instantiate("primer_contrato", &ink_e2e::bob(), &mut constructor)
                .submit()
                .await
                .expect("instantiate failed");
            let mut call_builder = contract.call_builder::<PrimerContrato>();

            let get = call_builder.get();
            let get_result = client.call(&ink_e2e::bob(), &get).dry_run().await?;
            assert!(matches!(get_result.return_value(), false));

            // When
            let flip = call_builder.flip();
            let _flip_result = client
                .call(&ink_e2e::bob(), &flip)
                .submit()
                .await
                .expect("flip failed");

            // Then
            let get = call_builder.get();
            let get_result = client.call(&ink_e2e::bob(), &get).dry_run().await?;
            assert!(matches!(get_result.return_value(), true));

            Ok(())
        }
    }
}



// implementacion del primerContrato o marketPlace
// Fn agregadas
// fn agregar_usuario_sistema  en linea 109

// implementacio del usuario 
// fn nuevo en linea 158   la implementacion  genero que tenga que agregar derive clone en el enum de rol y hacer el mismo pub 
// esto esta comentado en linea  56 y 58 no se que tan valido es poner el rol como pub  pero sino no funca

// fn publicacion de productos  la verdad esta fn esta para detonar pero capaz  sirve 
// la struct de producto paso a pub poque si no explota el programa  lo mismo que el rol no se que tan valido es esto 
//se agregaron derive clone arriba de algunas struct 