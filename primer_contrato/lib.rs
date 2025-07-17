#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(non_local_definitions)]

#[ink::contract]
mod primer_contrato {
    use ink::storage::Mapping;
    use ink::storage::StorageVec;
    use ink::prelude::vec::Vec;
    use ink::prelude::string::{String, ToString};

/////////////////////////// SISTEMA ///////////////////////////
    /// Struct que hace de "sistema". Encargado de persistir los datos. 
    /// Los usuarios se almacenan en un Mapping. La clave es el AccountId, y su contenido los datos del usuario.
    /// Las publicaciones realizadas se almacenan en un StorageVec, cuyo contenido es una tupla con el id de la publicación, y los datos de la misma. (id, publicacion).
    /// Los productos se almacenan en un Mapping. Donde la clave es el id del producto. y su contenido es una tupla que contiene los datos del producto y el stock de éste. <id, (producto, stock)>. 
    /// Las ordenes de compra se almacenan en un StorageVec. Cuyo contenido es una tupla con el id de la orden y los datos de la misma. (id, orden)   
    /// Por último se encuentra la dimensión lógica de "historial_productos" utilizada para definir la id de los productos que se agregan al sistema. 
    #[ink(storage)]
    pub struct PrimerContrato {
        usuarios: Mapping<AccountId, Usuario>,
        historial_publicaciones: StorageVec<(u32, Publicacion)>, 
        historial_productos: Mapping<u32, (Producto, u32)>, 
        historial_ordenes_de_compra: StorageVec<(u32, OrdenCompra)>,   
        dimension_logica_productos: u32, 
    }
    impl PrimerContrato {

        #[ink(constructor)]
        /// Contructor del sistema. Inicializa todo en default/new, y la dimensión logica de los productos en cero. 
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
        /// La función agregar_usuario_sistema se encarga de registrar el usuario en mi sistema (se almacena en "usuarios"). 
        /// Solo será registrado si su ID no se halla en mi Mapping de usuarios. 
        pub fn agregar_usuario_sistema(&mut self, nombre: String, apellido: String, direccion: String, email: String, rol: Rol) -> Result <(), String>{
            let account_id = self.env().caller();
            self.priv_agregar_usuario_sistema(account_id, nombre, apellido, direccion, email, rol)
        }
        fn priv_agregar_usuario_sistema(&mut self, account_id: AccountId, nombre: String, apellido: String, direccion: String, email: String, rol: Rol) -> Result <(), String>{
            if self.usuarios.get(account_id).is_some(){
                Err("El usuario ya esta registrado.".to_string())
            } else {
                let usuario = Usuario::nuevo(account_id, nombre, apellido, direccion, email, rol);
                self.usuarios.insert(account_id, &usuario); 
                Ok(())
            }
        }

        /// La función cargar_producto se encarga de registrar el producto en mi sistema (se almacena en "historial_productos"). 
        /// Se comprueba que el usuario que invoca la función esté registrado en mi sistema. Si lo está, se comprueba su rol. Si el usuario tiene rol vendedor u ambos, se le asigna un id al producto y luego es agregado a su estructura correspondiente. 
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

        /// La función "crear_publicacion" se encarga de crear la publicación y luego registrarla en mi sistema (se almacena en "historial_publicaciones"). 
        /// Primero se comprueba que el usuario que invocó la función exista en mi sistema. En caso de no estar se retorna un error.
        /// Si el usuario existe, se asigna el id de la publicación, se calcula su precio final, luego se crea la publicación delegando su creación al usuario.
        /// Una vez creada la publicación es agregada a mi sistema ("historial_publicaciones"). 
        /// Luego el usuario se reinserta en mis sistema debido a que en su método "crear_publicación" su estado fue modificado.
        /// Finalmente se retorna Ok, si todo ha salido bien. Indicando que la operación fue un éxito. 
        #[ink(message)]
        pub fn crear_publicacion(&mut self, productos_a_publicar: Vec<(u32, u32)>) -> Result<(), String> {
            let account_id = self.env().caller();
            self.priv_crear_publicacion(account_id, productos_a_publicar)
        }
        fn priv_crear_publicacion(&mut self, account_id: AccountId, productos_a_publicar: Vec<(u32, u32)>) -> Result<(), String> {
            if let Some(mut usuario) = self.usuarios.get(account_id){
                for (_id, cantidad) in productos_a_publicar.clone(){
                    if cantidad == 0 {
                        return Err("Un producto tiene cantidades no validas.".to_string())
                    }
                }
                self.hay_stock_suficiente(productos_a_publicar.clone())?;
                let id_publicacion = self.historial_publicaciones.len();
                let precio_final = self.calcular_precio_final(productos_a_publicar.clone())?;
                let publicacion = usuario.crear_publicacion(productos_a_publicar.clone(), precio_final, id_publicacion, account_id)?;
                self.descontar_stock(productos_a_publicar)?;
                self.historial_publicaciones.push(&(id_publicacion, publicacion));
                self.usuarios.insert(account_id, &usuario);
                Ok(())
            }
            else {
                Err("No existe el usuario.".to_string())
            }

        }
        
        
        #[ink(message)]
        /// La función "visualizar_productos_de_publicación" se encarga de retornar todos los datos de una publicación especifica (id) recibida por parámetro. 
        /// Recorro todo mi StorageVec en busca de la publicación deseada. Hasta encontrar la id correspondiente al producto buscado o hasta terminar de iterar sobre él. Si termina la iteración significa que no se encontró el producto con la id recibida, entonces terminando la función y retornando un error. 
        /// Si se halló el producto se clona los datos y son retornados en un Ok, indicando que la operación de búsqueda fue existosa. 
        pub fn visualizar_productos_de_publicacion(&self, id_publicacion: u32) -> Result<Publicacion, String> {
            self.priv_visualizar_productos_de_publicacion(id_publicacion)
        }
        fn priv_visualizar_productos_de_publicacion(&self, id_publicacion: u32) -> Result<Publicacion, String> {
            for i in 0..self.historial_publicaciones.len() {
                if let Some((id, publicacion)) = self.historial_publicaciones.get(i) {
                    if id == id_publicacion { 
                        return Ok(publicacion.clone());
                    }
                }
            }
            Err("No se encontro la publicacion.".to_string())
        }

        #[ink(message)]
        /// La función "modificar_rol" permite al usuario cambiar su rol al recibido por parametro. 
        /// Primero se comprueba que el usuario esté en mi sistema. En caso de no estarlo, se retorna un error. 
        /// Si el usuario se encuentra en mi sistema, se delega el cambio de estado al usuario. Si el rol recibido por parametro, es igual al que posee el usuario se retorna error.
        /// Si el rol fue efectivamente modificado se vuelve a insertar en el Mapping de usuarios del sistema debido a que su estado se alteró. Retornando Ok, indicando el éxito de la operación.
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
        /// La función crear_orden_de_compra se encarga de crear una orden de compra de una publicación recibida por parametro.
        /// Primero se comprueba que el usuario que invoca la función se encuentra en mi sistema. En caso de no estarlo, se retorna error.
        /// Luego se comprueba si el stock de la publicación es suficiente con el que posee el vendedor de los productos indicados en la publicación. Si no hay suficiente, se cancela la creación de la orden, retornando el error correspondiente.
        /// Se crea el id de la orden.
        /// Se delega la creación de la orden al usuario. Si la creación de la orden fue un éxito se sigue, sino se retorna el error apropidado. 
        /// Se descuenta el stock de los productos vendidos. 
        /// Se agrega la orden de compra a mi sistema.
        /// Por último se reinserta el usuario debido a que su estado interno ha sido modificado en el transcurso de la función. Retornando un Ok, indicando el éxito de la operación. 
        pub fn crear_orden_de_compra(&mut self, id_publicacion: u32) -> Result<(), String>{
            let account_id = self.env().caller();
            self.priv_crear_orden_de_compra(account_id, id_publicacion)
        }
        fn priv_crear_orden_de_compra(&mut self, account_id: AccountId, id_publicacion: u32) -> Result<(), String>{
            if let Some(mut usuario) = self.usuarios.get(account_id){
                let publicacion = self.visualizar_productos_de_publicacion(id_publicacion)?;
                let vendedor_de_la_orden = self.usuarios.get(publicacion.id_vendedor).unwrap();
                if account_id == vendedor_de_la_orden.id_usuario{
                    return Err("El usuario no puede comprar sus propias publicaciones.".to_string())
                }
                if vendedor_de_la_orden.rol == Rol::Comp {
                    return Err("La publicacion ya no se encuentra disponible.".to_string())
                }
                let id_orden = self.historial_ordenes_de_compra.len();
                let orden_de_compra = usuario.crear_orden_de_compra(id_orden, publicacion.clone(), account_id)?;
                self.historial_ordenes_de_compra.push(&(id_orden, orden_de_compra));
                self.usuarios.insert(account_id, &usuario);
                Ok(())  
            }
            else {
                Err("No existe el usuario.".to_string())
            }
        }
        
        /// Funcion para enviar una compra.
        /// Revisa que el usuario este cargado en sistema.
        /// Revisa que la orden de compra este cargada en sistema.
        /// Llama a la funcion de Usuario.
        /// Sobreescribe la orden en el sistema.
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
                            if orden_de_compra.estado != EstadoCompra::Pendiente{
                                return Err("El producto no puede ser enviado.".to_string());
                            }
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
        
        /// Funcion para recibir una compra.
        /// Revisa que el usuario este cargado en sistema.
        /// Revisa que la orden de compra este cargada en sistema.
        /// Revisa que la orden no haya sido enviada.
        /// Llama a la funcion de Usuario.
        /// Sobreescribe la orden en el sistema. 
        #[ink(message)]
        pub fn recibir_compra(&mut self, id_orden: u32) -> Result<(), String>{
            let account_id = self.env().caller();
            self.priv_recibir_compra(account_id, id_orden)
        }
        fn priv_recibir_compra(&mut self, account_id: AccountId, id_orden: u32) -> Result<(), String>{ 
            if let Some(usuario) = self.usuarios.get(account_id){
                for i in 0..self.historial_ordenes_de_compra.len() {
                    if let Some((id, mut orden_de_compra)) = self.historial_ordenes_de_compra.get(i){
                        if id == id_orden {
                            if orden_de_compra.estado != EstadoCompra::Enviado{
                                return Err("El producto todavia no fue enviado.".to_string());
                            }
                            usuario.recibir_compra(id_orden)?;
                            orden_de_compra.estado = EstadoCompra::Recibido;
                            let _ = self.historial_ordenes_de_compra.set(i, &(id, orden_de_compra));
                            return Ok(())
                        }
                    }
                }
                Err("No existe la orden buscada.".to_string())
            }
            else {
                Err("No existe el usuario.".to_string())
            }
        }

        //  Fn calcular_precio final  va a recibir los productos de la publicacion y va a prodecer a realizar 
        //  la suma de los valores de los productos
        //  esto retorna el valor total 
        //  y se chequea que no ocurra overflow como caso de error  
        fn calcular_precio_final(&self, productos_publicados: Vec<(u32, u32)>) -> Result<u32, String>{
            let mut total: u32 = 0;
            for (id, cantidad) in productos_publicados{
                if let Some((producto, _stock)) = self.historial_productos.get(id){
                    total = total.checked_add(producto.precio.checked_mul(cantidad)
                    .ok_or("Overflow al multiplicar precio por cantidad.")?)
                    .ok_or("Overflow al acumular el total.")?;
                }
            }
            Ok(total)
        }

      
        // Fn hay_stock_suficiente  recibe los productos y las cantidades  en un vec  y va a devolver si hay stock suficente o no 
        //  se va a recorrer el vector de productos buscando los productos para descontrar el stock y pueden haber los sigientes errores   // 
        // errores 
        // se encuentra el producto si hay stock continua, pero si no hay stock suficiente y devuelve error 
        // el producto puede no estas cargado en el sistema historial de productos 
        fn hay_stock_suficiente(&self, productos_cantidades: Vec<(u32, u32)>) -> Result<(), String>{
            for (id, cantidad) in productos_cantidades{
                if let Some((_producto, stock)) = self.historial_productos.get(id){
                    if (stock < cantidad) | (stock == 0){
                        return Err("No hay stock suficiente.".to_string())
                    }
                }
                else{
                    return Err("No se encontro el producto.".to_string())
                }
            }
            Ok(())
        }
        
        /// Funcion para descontar el stock en sistema de un cierto producto. Llamada dentro de la funcion "crear_orden_de_compra"
        /// Revisa que el producto este cargado en sistema.
        /// Resta el stock. Siempre habra stock minimo suficiente ya que se revisara con la funcion "hay_stock_suficiente".
        /// Sobreescribe el producto con su nuevo stock.
        fn descontar_stock(&mut self, productos_cantidades: Vec<(u32, u32)>) -> Result<(), String>{
            for (id, cantidad) in productos_cantidades{
                if let Some ((producto, mut stock)) = self.historial_productos.get(id){
                    stock = stock.checked_sub(cantidad).ok_or("Error al restar stock")?;
                    self.historial_productos.insert(id, &(producto, stock));
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
    /// Struct que almacena la información de un usuario. 
    /// id_usuario almacena el id.
    /// nombre almacena su nombre.
    /// apellido almacena su apellido.
    /// dirección almacena su dirección.
    /// email almacena su email.
    /// rol almacena el rol que tiene el usuario. Éste puede ser: Comp (comprador), Vend (vendedor), Ambos. 
    /// datos_comprador almacena toda la información correspondiente al rol comprador. El Option será Some cuando éste posea el rol Comp u Ambos. Si en algún momento deja de serlo, el Option seguirá en Some con toda la información.  
    /// datos_vendedor almacena toda la información correspondiente al rol vendedor. El Option será Some cuando éste posea el rol Vend u Ambos. Si en algún momento deja de serlo, el Option seguirá en Some con toda la información.  
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
        
        /// Funcion que permite crear un nuevo usuario, cargandole los datos correspondientes a este.
        /// Revisa el lol del usuario e inicaliza los datos correspondientes:
        /// Si es Comp -> datos_comprador = Some()
        /// Si es Vend -> datos_vendedor = Some()
        /// Si es Ambos -> datos_comprador = Some(), datos_vendedor = Some()
        pub fn nuevo(id: AccountId,nombre: String,apellido: String,direccion: String,email: String,rol: Rol) -> Usuario {
            Usuario {
                id_usuario: id,
                nombre,
                apellido,
                direccion,
                email,
                rol: rol.clone(),
                datos_comprador: match rol {
                    Rol::Comp | Rol::Ambos => Some(Comprador { 
                        ordenes_de_compra: Vec::new(),
                        reputacion_como_comprador: Vec::new(),
                    }),
                    _ => None,
                },
                datos_vendedor: match rol {
                    Rol::Vend | Rol::Ambos => Some(Vendedor {
                        productos: Vec::new(),
                        publicaciones: Vec::new(),
                        reputacion_como_vendedor: Vec::new(),
                    }),
                    _ => None,
                },
            }
        }

        
        /// Funcion que permite crear una publicacion. Llamada por sistema.
        /// Revisa que el usuario tenga el rol Vend o Ambos
        /// Llama a la funcion de Vendedor
        fn crear_publicacion(&mut self, productos_a_publicar: Vec<(u32, u32)>, precio_final: u32, id_publicacion: u32, id_vendedor: AccountId) -> Result<Publicacion, String>{  //productos_a_publicar = Vec<(id, cantidad)>
            if self.rol == Rol::Comp {
                Err("El usuario no es vendedor.".to_string())
            }
            else {
                Ok(self.datos_vendedor.as_mut().expect("Error con los datos del vendedor.").crear_publicacion(productos_a_publicar, precio_final, id_publicacion, id_vendedor))
            } 
        }

        
        /// Funcion que permite modificar el rol de un usuario. Llamada por sistema.
        /// Revisa que el rol a cambiar sea valido
        /// Si habia datos previos (en caso de haber cambiado de rol anteriormente) los usa, sino inicializa los campos
        fn modificar_rol(&mut self, nuevo_rol: Rol) -> Result<(), String>{
            if nuevo_rol == self.rol {
                Err("El usuario ya posee ese rol.".to_string())
            }
            else {
                self.rol = nuevo_rol.clone();
                match nuevo_rol{
                    Rol::Comp => {
                        if self.datos_comprador.is_none(){
                            let ordenes_de_compra = Vec::new();
                            let reputacion_como_comprador = Vec::new();
                            self.datos_comprador = Some(Comprador{
                                ordenes_de_compra,
                                reputacion_como_comprador,
                            });
                        }
                    }

                    Rol::Vend => {
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

                    Rol::Ambos => {
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
        
        
        /// Funcion para crear una orden de compra. Llamada por sistema.
        /// Revisa que el usuario tenga el rol de Comp o Ambos.
        /// Llama a la funcion de Comprador
        fn crear_orden_de_compra(&mut self, id_orden: u32, publicacion: Publicacion, id_comprador: AccountId) -> Result<OrdenCompra, String>{
            if self.rol == Rol::Vend{
                Err("El usuario no esta autorizado para realizar una compra. ERROR: No posee el rol comprador.".to_string())
            }
            else{
                Ok(self.datos_comprador.as_mut().expect("No hay datos del comprador.").crear_orden_de_compra(id_orden, publicacion, id_comprador))
            }
        }

        /// Funcion para enviar una compra. Llamada por sistema.
        /// Revisa que el usuario tenga el rol Vend o Ambos.
        /// Llama a la funcion de Vendedor.
        fn enviar_compra(&self, id_publicacion: u32) -> Result<(), String>{
            if self.rol == Rol::Comp{
                Err("El usuario no posee el rol de vendedor.".to_string())
            }
            else{
                self.datos_vendedor.as_ref().expect("No hay datos del vendedor.").enviar_compra(id_publicacion)
            }
        }

        /// Funcion para recibir una compra. Llamada por sistema.
        /// Revisa que el usuario tenga el rol Comp o Ambos.
        /// Llama a la funcion de Comprador.
        fn recibir_compra(&self, id_orden: u32) -> Result<(), String>{
            if self.rol == Rol::Vend{
                Err("El usuario no posee el rol de comprador.".to_string())
            }
            else{
                self.datos_comprador.as_ref().expect("No hay datos del comprador.").recibir_compra(id_orden)
            }
        }
    }



/////////////////////////// COMPRADOR ///////////////////////////

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    /// Struct donde se encuentran los datos de aquel usuario con rol Comprador. 
    /// ordenes_de_compra, es un Vec que almacena los IDs de cada orden realizada por el comprador. 
    /// reputacion_como_comprador, es un Vec que almacena las califaciones recibidas por vendedores. 
    struct Comprador{
        ordenes_de_compra: Vec<u32>,
        reputacion_como_comprador: Vec<u8>, 
    }
    impl Comprador{
        /// Esta fn crear orden compra es llamada por usuario y recibe estos datos:
        ///     Id de la orden, informarcion de la publicacion, y ademas el id del comprador 
        /// Se guarda el id de la orden
        /// Genera la orden de comrpra y la devuelve al sistema 
        fn crear_orden_de_compra(&mut self, id_orden: u32, publicacion: Publicacion, id_comprador: AccountId) -> OrdenCompra{
            self.ordenes_de_compra.push(id_orden);
            OrdenCompra::crear_orden_de_compra(id_orden, publicacion, id_comprador)
        }

        /// esta fn recibir_compra es llamada por usuario y recibe estos datos:=
        /// compra indica si el id de la orden esta en el vector de ordenes de compra perteneciente al comprador 
        /// en caso de no estar  genera el error
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
    /// Struct que contiene la información del usuario con rol vendedor. 
    /// productos, es un Vec con las ids de los productos de su propiedad. 
    /// reputacion_como_vendedor, es un Vec que almacena las califaciones recibidas por compradores. 
    struct Vendedor{
        productos: Vec<u32>,
        publicaciones: Vec<u32>,
        reputacion_como_vendedor: Vec<u8>, 
    }
    impl Vendedor{

        // Esta fn crear_publicacion es llamada por usuario recibe como datos  los productos que va a tener la publicacion (productos a publicar)
        // el precio final / coste de todos los productos juntos 
        // el id que tendra la publicacion
        // genera la publicacion 
        // el vendedor se guarda el  id_publicacion que se genero y se devuelve la publicacion generada para que se guarde en el sistema  
        fn crear_publicacion(&mut self, productos_a_publicar: Vec<(u32, u32)>, precio_final: u32, id_publicacion: u32, id_vendedor: AccountId) -> Publicacion { 
            let publicacion = Publicacion::crear_publicacion(productos_a_publicar, precio_final, id_publicacion, id_vendedor);
            self.publicaciones.push(id_publicacion);
            publicacion
        }


        // Esta fn enviar_compra es llamada por usuario recibe como dato  el id de la publicacion 
        // si el id se encuentra en  las publicaciones del vendenderor devuelve ok // exito 
        // en caso contrario  la pubiclacion no seria  de ese vendedor 
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

    #[derive(Debug, Clone, PartialEq)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]                            
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    /// Enum utulizado para definir el rol de un usuario. 
    /// Ambos (comprador y vendedor).
    /// Comp (comprador).
    /// Vend (vendedor).
    pub enum Rol {
        Ambos,
        Comp, 
        Vend,
    }




/////////////////////////// PUBLICACION ///////////////////////////

    #[derive(Clone, Debug)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    /// Struct que contiene la información de una publicacion. 
    /// id, guarda la id de la publicación. 
    /// productos, es un Vec que contiene tuplas cuyos campos son, el id de cada producto publicado y la cantidad de unidades publicadas de ese mismo producto. (id producto, cantidad del producto)
    /// precio_final, es la suma de, el precio de cada producto multiplicado por la cantidad de unidades del mismo. 
    /// id_vendedor, es el id del vendedor que realizó la publicación. 
    pub struct Publicacion{
        id: u32,
        productos: Vec<(u32, u32)>,
        precio_final: u32,
        id_vendedor:AccountId
    }

    /// Funcion para crear y devolver una publicacion. Llamada por Vendedor.
    /// Crea la publicacion con los datos pasados por parametro.
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
    /// Struct que contiene la información de un producto.
    /// id, almacena el id del producto. Utilizado como clave en el "historial_productos" del sistema. 
    /// nombre, almacena el nombre del producto.
    /// descripcion, almacena la descripción de un producto.
    /// precio, almacena el precio del producto.
    /// categoria, almacena la categoria. La categoria al ser un String, se sanitiza previo a ser cargada. 
    pub struct Producto{
        id: u32,
        nombre: String,
        descripcion: String,
        precio: u32,
        categoria:String,
    }
    impl Producto{
        /// Funcion para crear y devolver un producto. Llamada por Sistema.
        /// Crea un producto con los datos pasados por parametro.
        /// Para el campo de categoria, sanitiza el String, dejando todo en minuscula y sin espacios (para evitar conflictos)
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
    /// Struct que contiene la información de una orden de compra.
    /// id, almacena la id de la orden de compra. 
    /// estado, almacena el estado de la compra. Éste puede ser: Pendiente, Enviado, Recibido, Cancelada. 
    /// cancelacion, es una tupla que almacena el pedido de cancelación, tanto del vendedor como del comprador. (vendedor, comprador)
    /// info_publicacion, tupla que almacena los datos de la publicación. (ID de la publicacion, Vec<(IDs de los productos, cantidades de ese producto)>, precio final de la publicacion, ID del Vendedor).
    /// id_comprador, almacena el id del comprador de la orden de compra. 
    /// calificaciones, es una tupla que indica si el vendor y/o comprador realizó la calificación a su contraparte. (vendedor, comprador)
    struct OrdenCompra{
        id: u32,
        estado: EstadoCompra,
        cancelacion: (bool, bool), 
        info_publicacion: (u32, Vec<(u32, u32)>, u32, AccountId),
        id_comprador:AccountId,
        calificaciones: (bool, bool),
    }
    impl OrdenCompra{
        
        /// Funcion para crear y devolver una orden de compra. Llamada por Comprador.
        /// Crea una orden con los datos pasados por parametros, tanto "cancelacion" como "calificaciones" se inicializan con los dos campos en false.
        fn crear_orden_de_compra(id_orden: u32, publicacion: Publicacion, id_comprador: AccountId) -> OrdenCompra{
            let id_publicacion = publicacion.id;
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
    /// Enum utilizado para indicar el estado de la orden de compra. 
    /// Pendiente (cuando la orden se crea).
    /// Enviado (cuando el vendedor envía los productos de la publicación).
    /// Recibido (cuando el comprador recibe los productos de la orden). 
    /// Cancelada (solo se asignará cuando ambas partes de la orden de compra cancelan la misma). 
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
                "Frank".into(),
                "Apellido".into(),
                "Dirección".into(),
                "frank@mail.com".into(),
                Rol::Vend,
            ).unwrap();

            // Asegurarse que el producto 42 no existe
            assert!(contrato.historial_productos.get(42).is_none());

            // Intenta publicar un producto inexistente
            let resultado = contrato.crear_publicacion(vec![(42, 1)]);
            assert!(resultado.is_err(), "Debería fallar por producto inexistente");

            let err = resultado.unwrap_err();
            assert_eq!(err, "No se encontro el producto.");
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
            let mensaje = result.unwrap_err();
            assert_eq!(mensaje, "El usuario no puede comprar sus propias publicaciones.");
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
            contrato.agregar_usuario_sistema("Vendedor".into(), "A".into(), "Dir".into(), "v@mail.com".into(), Rol::Vend).unwrap();
            contrato.cargar_producto("P5".into(), "Desc".into(), 100, "cat".into(), 1).unwrap();

            // Intentar crear publicación con más cantidad que stock disponible
            let resultado_pub = contrato.crear_publicacion(vec![(1, 2)]);
            assert!(resultado_pub.is_err(), "No debería poder crear publicación con cantidad mayor al stock");
            assert_eq!(resultado_pub.unwrap_err(), "No hay stock suficiente.");
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

        #[ink::test]
        fn test_modificar_rol_a_comprador_agrega_datos_comprador_si_no_existen() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let mut contrato = PrimerContrato::default();

            // Registrar usuario como vendedor
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            contrato.agregar_usuario_sistema(
                "Alice".to_string(),
                "Apellido".to_string(),
                "Direccion".to_string(),
                "alice@mail.com".to_string(),
                Rol::Vend,
            ).unwrap();

            // Modificar rol a Comp
            let result = contrato.modificar_rol(Rol::Comp);
            assert!(result.is_ok(), "El cambio de rol a Comp debería funcionar");

            let usuario = contrato.usuarios.get(accounts.alice).unwrap();
            assert_eq!(usuario.rol, Rol::Comp);
            assert!(usuario.datos_comprador.is_some(), "Debe tener datos_comprador");
            assert!(usuario.datos_vendedor.is_some(), "Debe conservar datos_vendedor previos");
        }

        #[ink::test]
        fn test_modificar_rol_a_vendedor_agrega_datos_vendedor_si_no_existen() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let mut contrato = PrimerContrato::default();

            // Registrar usuario como comprador
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            contrato.agregar_usuario_sistema(
                "Bob".to_string(),
                "Apellido".to_string(),
                "Direccion".to_string(),
                "bob@mail.com".to_string(),
                Rol::Comp,
            ).unwrap();

            // Modificar rol a Vend
            let result = contrato.modificar_rol(Rol::Vend);
            assert!(result.is_ok(), "El cambio de rol a Vend debería funcionar");

            let usuario = contrato.usuarios.get(accounts.bob).unwrap();
            assert_eq!(usuario.rol, Rol::Vend);
            assert!(usuario.datos_vendedor.is_some(), "Debe tener datos_vendedor");
            assert!(usuario.datos_comprador.is_some(), "Debe conservar datos_comprador previos");
        }
    }
}
