#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(non_local_definitions)]

pub use self::primer_contrato::{
    PrimerContratoRef,
};

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
        /// La función agregar_usuario_sistema se encarga de registrar un usuario en mi sistema. 
        /// 
        /// Errores posibles: cuando el usuario ya está registrado. 
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

        #[ink(message)]
        pub fn get_dimension_logica(&self) -> u32{
            self.dimension_logica_productos
        }
        
        #[ink(message)]
        /// La función "modificar_rol" permite al usuario cambiar su rol al recibido por parametro. 
        pub fn modificar_rol(&mut self, nuevo_rol: Rol) -> Result<(), String>{
            let account_id = self.env().caller();
            self.priv_modificar_rol(account_id, nuevo_rol)
        }
        fn priv_modificar_rol(&mut self, account_id: AccountId, nuevo_rol: Rol) -> Result<(), String>{
            let mut usuario = self.buscar_usuario(account_id)?;
            usuario.modificar_rol(nuevo_rol)?;
            self.usuarios.insert(account_id, &usuario);
            Ok(())
        }

        /// La función cargar_producto se encarga de registrar un producto en mi sistema.
        /// 
        /// Errores posibles: el precio recibido por parametro es 0; 
        /// el stock recibido por parametro es 0; 
        /// si el usuario tiene rol Comp.
        #[ink(message)]
        pub fn cargar_producto(&mut self, nombre: String, descripcion: String, precio: u32, categoria: String, stock: u32) -> Result<(), String>{
            let account_id = self.env().caller();
            self.priv_cargar_producto(account_id, nombre, descripcion, precio, categoria, stock)
        }
        fn priv_cargar_producto(&mut self, account_id: AccountId, nombre: String, descripcion: String, precio: u32, categoria: String, stock: u32) -> Result<(), String>{
            if precio == 0 { //<---- Desde. Correccion punto 12. 12/08
                return Err("Precio no valido".to_string())
            }
            if stock == 0{
                return Err("Stock no valido".to_string())
            } //<---- Hasta. 
            let mut usuario = self.buscar_usuario(account_id)?;
            if (usuario.rol == Rol::Vend) | (usuario.rol == Rol::Ambos){
                self.dimension_logica_productos = self.dimension_logica_productos.checked_add(1).ok_or("Error al sumar.")?;
                self.historial_productos.insert(self.dimension_logica_productos, &(Producto::cargar_producto(self.dimension_logica_productos, nombre, descripcion, precio, categoria), stock));
                usuario.cargar_producto(self.dimension_logica_productos)?;
                self.usuarios.insert(account_id, &usuario);
                Ok(())
            }
            else {
                Err("El usuario no tiene permisos para cargar productos. No es vendedor.".to_string())
            }
        }

        /// La función "visualizar_productos_propios" devuelve un Vector de tuplas donde cada tupla tiene,
        /// en la posición 0, el ID del producto, y en la posición 1 el stock.
        #[ink(message)]
        pub fn visualizar_productos_propios(&self) -> Result<Vec<(u32, u32)>, String>{
            let account_id = self.env().caller();
            self.priv_visualizar_productos_propios(account_id)
        }
        fn priv_visualizar_productos_propios(&self, account_id: AccountId) -> Result<Vec<(u32, u32)>, String>{
            let mut usuario = self.buscar_usuario(account_id)?;
            usuario.es_vendedor_ambos()?;
            let lista_de_productos = usuario.lista_de_productos()?;
            let mut productos = self.cargar_productos_propios(account_id, lista_de_productos);
            Ok(productos)
        }

        /// La función "crear_publicacion" se encarga de crear la publicación y luego registrarla en mi sistema. 
        /// Recibe un Vector de tuplas donde la posición 0 es el ID del producto, y la posición 1 es la cantidad a publicar de ese producto.
        /// 
        /// Errores posibles: cuando la cantidad de un producto a publicar es 0. 
        #[ink(message)]
        pub fn crear_publicacion(&mut self, productos_a_publicar: Vec<(u32, u32)>) -> Result<(), String> {
            let account_id = self.env().caller();
            self.priv_crear_publicacion(account_id, productos_a_publicar)
        }
        fn priv_crear_publicacion(&mut self, account_id: AccountId, productos_a_publicar: Vec<(u32, u32)>) -> Result<(), String> {
            let mut usuario = self.buscar_usuario(account_id)?;
            for (id, cantidad) in productos_a_publicar.clone(){
                if cantidad == 0 {
                    return Err("Un producto tiene cantidades no validas.".to_string())
                }
                usuario.verificar_propiedad_producto(id)? // <------ Correccion Punto 12. 12/08
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
        
        /// La función "visualizar_productos_de_publicación" se encarga de retornar todos los datos de una publicación (ID) recibida por parámetro. Devolviendo un tipo de dato Publicacion.  
        /// 
        /// Errores posibles: cuando el ID recibido por parametro no se halla en mi sistema (historial_publicaciones).
        #[ink(message)]
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
         
        /// La función "crear_orden_de_compra" se encarga de crear una orden de compra de una publicación (ID) recibida por parametro.
        /// 
        /// Errores posibles: cuando la publicación ya no está disponible (boolean de Publicacion = false);
        /// Cuando el usuario que quiere comprar una publicación, y es también el vendedor de la misma; 
        /// Cuando el usuario que creó la publicación y luego cambia de rol a Comp. 
        #[ink(message)]
        pub fn crear_orden_de_compra(&mut self, id_publicacion: u32) -> Result<(), String>{
            let account_id = self.env().caller();
            self.priv_crear_orden_de_compra(account_id, id_publicacion)
        }
        fn priv_crear_orden_de_compra(&mut self, account_id: AccountId, id_publicacion: u32) -> Result<(), String>{
            let mut usuario = self.buscar_usuario(account_id)?;
            let mut publicacion = self.visualizar_productos_de_publicacion(id_publicacion)?;
            if publicacion.disponible{ //Agregado el 14/08
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
                if self.puede_restockear(publicacion.clone()){ //Agregado el 14/08
                    self.descontar_stock(publicacion.productos)?;
                }
                else {
                    publicacion.disponible = !publicacion.disponible;
                    let pos = self.devolver_posicion_publicacion(id_publicacion)?;
                    self.historial_publicaciones.set(pos, &(id_publicacion, publicacion));
                }
                Ok(())  
            }
            else { //Agregado el 14/08
                Err("La publicacion ya no tiene stock".to_string())
            }
        }
        
        /// La función "cancelar_compra" se encarga de cancelar una compra.
        #[ink(message)]
        pub fn cancelar_comprar(&mut self, id_orden: u32) -> Result<(), String>{
            let account_id = self.env().caller();
            self.priv_cancelar_compra(account_id, id_orden)
        }
        fn priv_cancelar_compra(&mut self, account_id: AccountId, id_orden: u32) -> Result<(), String>{
            let mut usuario = self.buscar_usuario(account_id)?;
            let mut datos_de_la_orden = self.buscar_orden(id_orden)?;
            self.se_puede_cancelar(datos_de_la_orden.estado.clone())?;
            let id_vendedor = datos_de_la_orden.info_publicacion.3;
            let id_comprador = datos_de_la_orden.id_comprador;
            let id_publicacion = datos_de_la_orden.info_publicacion.0; 
            let rol = usuario.comprobar_rol(id_vendedor, id_comprador)?;
            if rol == Rol::Comp{
                datos_de_la_orden.cancelar_compra_comprador()?;
            }
            else {
                datos_de_la_orden.cancelar_compra_vendedor()?;
                self.devolver_productos(id_publicacion)?;
            }
            self.actualizar_ordenes(datos_de_la_orden, id_orden)?;
            return Ok(());
        }

        /// Funcion para enviar una compra.
        /// 
        /// Errores posibles: cuando el estado de la compra no es pendiente; 
        /// cuando el ID de la orden recibida por parametro no se halla en mi sistema (historial_ordenes_compra).
        #[ink(message)]
        pub fn enviar_compra(&mut self, id_orden: u32) -> Result<(), String>{
            let account_id = self.env().caller();
            self.priv_enviar_compra(account_id, id_orden)
        }
        fn priv_enviar_compra(&mut self, account_id: AccountId, id_orden: u32) -> Result<(), String>{
            let mut usuario = self.buscar_usuario(account_id)?;
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
        
        /// Funcion para recibir una compra.
        /// 
        /// Errores posibles: cuando el estado de la orden de compra no es enviado;
        /// cuando el ID de la orden recibida por parametro no se halla en mi sistema (historial_ordenes_compra).
        #[ink(message)]
        pub fn recibir_compra(&mut self, id_orden: u32) -> Result<(), String>{
            let account_id = self.env().caller();
            self.priv_recibir_compra(account_id, id_orden)
        }
        fn priv_recibir_compra(&mut self, account_id: AccountId, id_orden: u32) -> Result<(), String>{ 
            let mut usuario = self.buscar_usuario(account_id)?;
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

        /// Función que se encarga de calificar a un usuario.
        /// 
        /// Errores posibles: cuando la calificación recibida por parametro se encuentra fuera de rango (rango = [1..5]).
        #[ink(message)]
        pub fn calificar (&mut self, id_orden:u32, calificacion: u8) -> Result<(), String>{
            let account_id = self.env().caller();
            self.priv_calificar(id_orden, calificacion, account_id)
        }
        fn priv_calificar(&mut self, id_orden:u32, calificacion: u8, account_id: AccountId) -> Result<(), String>{
            if (calificacion < 1) | (calificacion > 5){ //Revisa que la calificacion este en rango
                return Err("El valor de la calificacion no es valido (1..5).".to_string())
            }
            let mut usuario = self.buscar_usuario(account_id)?;
            let mut orden_de_compra = self.buscar_orden(id_orden)?;
            let id_comprador = orden_de_compra.id_comprador; //guarda la id del comprador y del vendedor
            let id_vendedor = orden_de_compra.info_publicacion.3;
            self.comprobar_estado_recibido(orden_de_compra.clone())?;
            self.calificar_segun_rol(calificacion, orden_de_compra, id_vendedor, id_comprador, usuario) 
        }

        /// La función "calcular_precio_final" se encarga de calcular el precio final de una publicación.
        /// Recibe los productos con sus cantidades de una publicación y retorna el precio final. 
        /// 
        /// Errores posibles: overflow en la suma/multiplicación. 
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

      
        /// La función "hay_stock_suficiente" se encarga de comprobar que el stock de mis productos almacenados sean suficientes y distintos de cero.
        /// Recibe un Vector de tuplas donde la posición cero es el ID del producto y la posición uno la cantidad de ese producto a publicar. 
        /// 
        /// Errores posibles: si el stock es menor a la cantidad a publicar o el stock de mi producto es cero;
        /// si el producto a publicar no se encuentra en mi sistema (historial_productos). 
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
        
        /// Función para descontar el stock de un producto. Recibe los productos (ID) con las cantidades a descontar. 
        /// 
        /// Errores posibles: cuando ocurre overflow/underflow al realizar la resta. 
        fn descontar_stock(&mut self, productos_cantidades: Vec<(u32, u32)>) -> Result<(), String>{
            for (id, cantidad) in productos_cantidades{
                if let Some ((producto, mut stock)) = self.historial_productos.get(id){
                    stock = stock.checked_sub(cantidad).ok_or("Error al restar stock")?;
                    self.historial_productos.insert(id, &(producto, stock));
                }
            }
            Ok(())
        }

        /// La función se encarga de cargar los productos (ID, stock) en un Vector. 
        /// Recibe un Vector con IDs de productos y retorna un Vector de tuplas con ID del producto, stock del producto.
        fn cargar_productos_propios(&self, account_id: AccountId, lista_de_productos: Vec<u32>) -> Vec<(u32, u32)>{
            let mut productos = Vec::new();
            for id_producto in lista_de_productos {
                let stock = self.historial_productos.get(id_producto).unwrap().1;
                let producto = (id_producto, stock);
                productos.push(producto);
            }
            productos
        }

        /// La función "puede_restockear" se encarga de comprobar si el stock de cada producto es mayor o igual al de una publicacion recibida por parametro.
        /// 
        /// Retornos posibles: falso en caso de que el stock sea insuficiente para reponer;
        /// verdadero cuando es posible realizar la reposición. 
        fn puede_restockear(&self, publicacion: Publicacion) -> bool{
            for (id, cantidad) in publicacion.productos{
                let stock = self.historial_productos.get(id).unwrap().1;
                if stock < cantidad{
                    return false;
                }
            }
            return true;
        }

        /// La función se encarga de devolver la posición en la que se encuentra una publicación en base a un ID recibido por parametro. 
        /// 
        /// Errores posibles: cuando la publicación no se encuentra en mi sistema (historial_publicaciones).
        fn devolver_posicion_publicacion(&self, id_publicacion: u32) -> Result<u32, String>{
            for i in 0..self.historial_publicaciones.len() {
                if let Some((id, publicacion)) = self.historial_publicaciones.get(i) {
                    if id == id_publicacion { 
                        return Ok(i);
                    }
                }
            }
            return Err("No se encontro publicacion".to_string());
        }

        /// La función se encarga de comprobar si es posible cancelar una compra. 
        /// 
        /// Errores posibles: cuando la compra no tiene estado Pendiente. 
        fn se_puede_cancelar(&self, estado_de_la_orden: EstadoCompra)-> Result<(), String>{
            if estado_de_la_orden != EstadoCompra::Pendiente{
                return Err("Ya no se puede cancelar la compra.".to_string())
            }
            else {
                return Ok(())
            }
        }

        /// La función se encarga de reponer los productos en la publicación o en el stock del producto que corresponda.
        fn devolver_productos(&mut self, id_publicacion: u32,) -> Result<(), String>{
            let mut publicacion = self.buscar_publicacion(id_publicacion)?;
            if publicacion.disponible {
                return self.aumentar_stock_productos(publicacion.productos);
            }
            else {
                publicacion.disponible = true;
                return self.actualizar_publicaciones(publicacion, id_publicacion);
            }
        }

        /// La función se encarga de aumentar el stock de cada producto recibido por parametro. 
        /// Recibe un Vector de tuplas donde la posición cero es el ID del producto y la posición uno es la cantidad a aumentar.
        /// 
        /// Errores posibles: overflow en la suma;
        /// un ID del Vector recibido por parametro no se encuentra en mi sistema (historial_productos). 
        fn aumentar_stock_productos(&mut self, productos_cantidades: Vec<(u32, u32)>) -> Result<(), String>{
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

        /// La función se encarga de pisar un valor del StorageVec "historial_publicaciones" en base a un ID de una publicación recibida por parametro.
        fn actualizar_publicaciones(&mut self, publicacion: Publicacion, id_publicacion: u32) -> Result<(), String>{
            let pos = self.devolver_posicion_publicacion(id_publicacion)?;
            self.historial_publicaciones.set(pos, &(id_publicacion, publicacion));
            return Ok(());
        }

        /// La función se encarga de pisar un valor del StorageVec "historial_ordenes_de_compra" en base a un ID de una orden recibida por parametro.
        fn actualizar_ordenes(&mut self, orden: OrdenCompra, id_orden: u32) -> Result<(), String>{
            let pos = self.devolver_posicion_orden_de_compra(id_orden)?;
            self.historial_ordenes_de_compra.set(pos, &(id_orden, orden));
            return Ok(());
        }

        /// La función se encarga de devolver una OrdenCompra en base a un ID recibido por parametro.
        /// 
        /// Errores posibles: cuando no se encuentra el ID de la orden en mi sistema (historial_ordenes_de_compra). 
        fn buscar_orden(&self, id_orden: u32) -> Result<OrdenCompra, String>{
            for i in 0..self.historial_ordenes_de_compra.len(){
                if let Some((id, ref orden)) = self.historial_ordenes_de_compra.get(i){
                    if id == id_orden{
                        return Ok(orden.clone());
                    }
                }
            }
            return Err("No se encontro la orden.".to_string());
        }

        /// La función se encarga de devolver una Publicación en base a un ID recibido por parametro. 
        /// 
        /// Errores posibles: cuando no se encuentra el ID de la publicación en mi sistema (historial_publicaciones).
        fn buscar_publicacion(&self, id_publicacion: u32) -> Result<Publicacion, String>{
            for i in 0..self.historial_publicaciones.len(){
                if let Some((id, ref publicacion)) = self.historial_publicaciones.get(i){
                    if id == id_publicacion{
                        return Ok(publicacion.clone());
                    }
                }
            }
            return Err("No se encontro la publicacion.".to_string());
        }

        /// La función se encarga de devolver la posición de una orden de compra en base a el ID recibido por parametro.
        /// 
        /// Errores posibles: cuando la orden con el ID recibido por parametro no se encuentra en mi sistema (historial_ordenes_de_compra).
        fn devolver_posicion_orden_de_compra(&self, id_orden: u32) -> Result<u32, String>{
            for i in 0..self.historial_ordenes_de_compra.len() {
                if let Some((id, orden)) = self.historial_ordenes_de_compra.get(i) {
                    if id == id_orden { 
                        return Ok(i);
                    }
                }
            }
            return Err("No se encontro la orden de compra.".to_string());
        }

        /// La función se encarga comprobar el estado de la compra. 
        /// 
        /// Errores posibles: cuando el estado de la compra es distinto de Recibido. 
        fn comprobar_estado_recibido(&self, orden: OrdenCompra) -> Result<(), String>{
            if orden.estado == EstadoCompra::Recibido{
                Ok(())
            }
            else{
                Err("La compra aun no fue recibida.".to_string())
            }
        }

        /// La función se encarga de calificar a un usuario según el rol que tuvo en la compra. 
        fn calificar_segun_rol(&mut self, calificacion: u8, mut orden_de_compra: OrdenCompra, id_vendedor: AccountId, id_comprador: AccountId, usuario: Usuario) -> Result<(), String>{
            let rol_del_usuario_en_compra = usuario.comprobar_rol(id_vendedor, id_comprador)?;
            let _ = self.ya_califico(rol_del_usuario_en_compra.clone(), orden_de_compra.clone())?;
            if rol_del_usuario_en_compra == Rol::Comp{
                self.calificar_vendedor(id_vendedor, calificacion)?;
                orden_de_compra.calificaciones.0 = true;
            }
            else {
                self.calificar_comprador(id_comprador, calificacion)?;
                orden_de_compra.calificaciones.1 = true;
            }
            let id_orden = orden_de_compra.id;
            self.actualizar_ordenes(orden_de_compra, id_orden)
        }

        /// La función se encarga de de comprobar si un usuario ya calificó una compra que realizó. 
        /// 
        /// Errores posibles: cuando el usuario ya calificó la compra anteriormente. 
        fn ya_califico(&self, rol: Rol, orden: OrdenCompra) -> Result<(), String>{
            if ((rol == Rol::Comp) & (orden.calificaciones.0)) | ((rol == Rol::Vend) & (orden.calificaciones.1)){
                return Err("El usuario ya califico.".to_string())
            }
            Ok(())
        }

        /// La función se encarga de registrar la puntuación recibida por parametro a un usuario con rol Vend.
        /// 
        /// Errores posibles: cuando el usuario no tiene los datos correspondientes a un vendedor cargados. 
        fn calificar_vendedor(&mut self, id_vendedor: AccountId, calificacion: u8) -> Result<(), String>{
            let mut vendedor = self.buscar_usuario(id_vendedor)?;
            if let Some(ref mut datos_vendedor) = vendedor.datos_vendedor{
                datos_vendedor.reputacion_como_vendedor.push(calificacion);
                self.actualizar_usuarios(vendedor);
                Ok(())
            }
            else{
                Err("El vendedor no tiene datos cargados.".to_string())
            }
        }

        /// La función se encarga de registrar la puntuación recibida por parametro a un usuario con rol Comp. 
        /// 
        /// Errores posibles: cuando el usuario no tiene los datos correspondientes a un comprador cargados. 
        fn calificar_comprador(&mut self, id_comprador: AccountId, calificacion: u8) -> Result<(), String>{
            let mut comprador = self.buscar_usuario(id_comprador)?;
            if let Some(ref mut datos_comprador) = comprador.datos_comprador{
                datos_comprador.reputacion_como_comprador.push(calificacion);
                self.actualizar_usuarios(comprador);
                Ok(())
            }
            else{
                Err("El comprador no tiene datos cargados.".to_string())
            }
        }

        /// La función se encarga de pisar un valor de mi Mapping "usuarios".  
        fn actualizar_usuarios(&mut self, usuario: Usuario){
            self.usuarios.insert(usuario.id_usuario, &usuario);
        }

        /// La función se encarga de devolver el Usuario correspondiente al ID recibido por parametro. 
        /// 
        /// Errores posibles: no se halla el ID en mi sistema (usuarios). 
        fn buscar_usuario(&self, id_usuario: AccountId) -> Result<Usuario, String>{
            if let Some(ref usuario) = self.usuarios.get(id_usuario){
                Ok(usuario.clone())
            }
            else {
                return Err("No se encontro el usuario.".to_string())
            }
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
    #[derive(Clone)]
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

        fn es_vendedor_ambos(&self) -> Result<(), String>{
            if self.rol == Rol::Comp{
                return(Err("El usuario es comprador.".to_string()))
            }
            Ok(())
        }

        fn es_comprador_ambos(&self) -> Result<(), String>{
            if self.rol == Rol::Vend{
                return(Err("El usuario es vendedor.".to_string()))
            }
            Ok(())
        }

        fn lista_de_productos(&self) -> Result<Vec<u32>, String>{
            if let Some(ref datos_vendedor) = self.datos_vendedor{
                if datos_vendedor.productos.is_empty(){
                    return Err("El vendedor no tiene productos.".to_string())
                }
                Ok(datos_vendedor.productos.clone())
            }
            else{
                return Err("El vendedor no tiene datos.".to_string())
            }
        }

        fn verificar_propiedad_producto(&self, id_producto: u32) -> Result<(), String>{
            if let Some(ref datos_vendedor) = self.datos_vendedor{
                if datos_vendedor.productos.contains(&id_producto){
                    return Ok(())
                }
                return Err("El vendedor no posee ese producto.".to_string())
            }
            else{
                return Err("El vendedor no tiene datos.".to_string())
            }
        }

        fn cargar_producto(&mut self, id_producto: u32) -> Result<(), String>{
            if let Some(ref mut datos_vendedor) = self.datos_vendedor{
                datos_vendedor.cargar_producto(id_producto);
                return Ok(())
            }
            else {
                return Err("El vendedor no tiene datos.".to_string())
            }
        }
        
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

        fn crear_publicacion(&mut self, productos_a_publicar: Vec<(u32, u32)>, precio_final: u32, id_publicacion: u32, id_vendedor: AccountId) -> Result<Publicacion, String>{  //productos_a_publicar = Vec<(id, cantidad)>
            if self.rol == Rol::Comp {
                Err("El usuario no es vendedor.".to_string())
            }
            else {
                Ok(self.datos_vendedor.as_mut().expect("Error con los datos del vendedor.").crear_publicacion(productos_a_publicar, precio_final, id_publicacion, id_vendedor))
            } 
        }

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
        
        fn crear_orden_de_compra(&mut self, id_orden: u32, publicacion: Publicacion, id_comprador: AccountId) -> Result<OrdenCompra, String>{
            if self.rol == Rol::Vend{
                Err("El usuario no esta autorizado para realizar una compra. ERROR: No posee el rol comprador.".to_string())
            }
            else{
                Ok(self.datos_comprador.as_mut().expect("No hay datos del comprador.").crear_orden_de_compra(id_orden, publicacion, id_comprador))
            }
        }

        fn enviar_compra(&self, id_publicacion: u32) -> Result<(), String>{
            if self.rol == Rol::Comp{
                Err("El usuario no posee el rol de vendedor.".to_string())
            }
            else{
                self.datos_vendedor.as_ref().expect("No hay datos del vendedor.").enviar_compra(id_publicacion)
            }
        }

        fn recibir_compra(&self, id_orden: u32) -> Result<(), String>{
            if self.rol == Rol::Vend{
                Err("El usuario no posee el rol de comprador.".to_string())
            }
            else{
                self.datos_comprador.as_ref().expect("No hay datos del comprador.").recibir_compra(id_orden)
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
    }



/////////////////////////// COMPRADOR ///////////////////////////

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    /// Struct donde se encuentran los datos de aquel usuario con rol Comprador. 
    /// ordenes_de_compra, es un Vec que almacena los IDs de cada orden realizada por el comprador. 
    /// reputacion_como_comprador, es un Vec que almacena las califaciones recibidas por vendedores. 
    #[derive(Clone)]
    struct Comprador{
        ordenes_de_compra: Vec<u32>,
        reputacion_como_comprador: Vec<u8>, 
    }
    impl Comprador{

        fn crear_orden_de_compra(&mut self, id_orden: u32, publicacion: Publicacion, id_comprador: AccountId) -> OrdenCompra{
            self.ordenes_de_compra.push(id_orden);
            OrdenCompra::crear_orden_de_compra(id_orden, publicacion, id_comprador)
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
    /// Struct que contiene la información del usuario con rol vendedor. 
    /// productos, es un Vec con las ids de los productos de su propiedad. 
    /// reputacion_como_vendedor, es un Vec que almacena las califaciones recibidas por compradores. 
    #[derive(Clone)]
    struct Vendedor{
        productos: Vec<u32>,
        publicaciones: Vec<u32>,
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

        fn cargar_producto(& mut self, id_producto: u32){
            self.productos.push(id_producto);
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
        id_vendedor:AccountId,
        disponible: bool,
    }

    impl Publicacion {
        fn crear_publicacion(productos_a_publicar: Vec<(u32, u32)>, precio_final: u32, id_publicacion: u32, id_vendedor: AccountId) -> Publicacion{
            Publicacion{
                id: id_publicacion,
                productos: productos_a_publicar,
                precio_final,
                id_vendedor,
                disponible: true,
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
        
        fn cancelar_compra_comprador(&mut self) -> Result<(), String>{
            if self.cancelacion.1 {
                return Err("La compra ya fue cancelada por el comprador anteriormente.".to_string());
            }
            else{
                self.cancelacion.1 = true;
                return Ok(());
            }
        }
        
        fn cancelar_compra_vendedor(&mut self) -> Result<(), String>{
            if self.cancelacion.0 {
                return Err("La compra ya fue cancelada.".to_string());
            }
            else {
                if self.cancelacion.1 {
                    self.cancelacion.0 = true;
                    return Ok(());
                }
                else {
                    return Err("El comprador no desea cancelar la compra.".to_string());
                }
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

//////////////////////////TEST/////////////////////////////////////
mod tests {
    use super::*;

    ///Función auxiliar para generar AccountID distintos en cada test 
    fn account(n: u8) -> AccountId {
        AccountId::from([n; 32])
    }

    #[ink::test]
    fn agregar_usuario_comp_exitoso() {
        let mut contrato = PrimerContrato::new();
        let acc = account(1);

        let res = contrato.priv_agregar_usuario_sistema(
            acc,
            "Juan".to_string(),
            "Pérez".to_string(),
            "Calle Falsa 123".to_string(),
            "juan@mail.com".to_string(),
            Rol::Comp,
        );

        assert!(res.is_ok(), "Debería registrar correctamente");
        let u = contrato.usuarios.get(acc).expect("Usuario no almacenado");
        assert!(u.datos_comprador.is_some(), "Para Rol::Comp debe inicializar datos_comprador");
        assert!(u.datos_vendedor.is_none(), "Para Rol::Comp no debe inicializar datos_vendedor");
    }

    #[ink::test]
    fn agregar_usuario_vend_exitoso() {
        let mut contrato = PrimerContrato::new();
        let acc = account(2);

        let res = contrato.priv_agregar_usuario_sistema(
            acc,
            "Ana".to_string(),
            "López".to_string(),
            "Av. Central".to_string(),
            "ana@mail.com".to_string(),
            Rol::Vend,
        );

        assert!(res.is_ok(), "Debería registrar correctamente");
        let u = contrato.usuarios.get(acc).expect("Usuario no almacenado");
        assert!(u.datos_comprador.is_none(), "Para Rol::Vend no debe inicializar datos_comprador");
        assert!(u.datos_vendedor.is_some(), "Para Rol::Vend debe inicializar datos_vendedor");
    }

    #[ink::test]
    fn agregar_usuario_ambos_exitoso() {
        let mut contrato = PrimerContrato::new();
        let acc = account(3);

        let res = contrato.priv_agregar_usuario_sistema(
            acc,
            "Luis".to_string(),
            "Gómez".to_string(),
            "Ruta 8".to_string(),
            "luis@mail.com".to_string(),
            Rol::Ambos,
        );

        assert!(res.is_ok(), "Debería registrar correctamente");
        let u = contrato.usuarios.get(acc).expect("Usuario no almacenado");
        assert!(u.datos_comprador.is_some(), "Para Rol::Ambos debe inicializar datos_comprador");
        assert!(u.datos_vendedor.is_some(), "Para Rol::Ambos debe inicializar datos_vendedor");
    }

    #[ink::test]
    fn agregar_usuario_duplicado_falla() {
        let mut contrato = PrimerContrato::new();
        let acc = account(4);

        assert!(contrato.priv_agregar_usuario_sistema(
            acc,
            "Sofía".to_string(),
            "Ibarra".to_string(),
            "Mitre 100".to_string(),
            "sofia@mail.com".to_string(),
            Rol::Vend,
        ).is_ok());

        let res = contrato.priv_agregar_usuario_sistema(
            acc,
            "Sofía".to_string(),
            "Ibarra".to_string(),
            "Mitre 100".to_string(),
            "sofia@mail.com".to_string(),
            Rol::Vend,
        );

        assert!(res.is_err(), "Debe devolver error por usuario duplicado");
        assert_eq!(res.unwrap_err(), "El usuario ya esta registrado.");
    }

    #[ink::test]
    fn modificar_rol_usuario_inexistente_falla() {
        let mut contrato = PrimerContrato::new();
        let acc = account(10);

        let res = contrato.priv_modificar_rol(acc, Rol::Vend);

        assert!(res.is_err(), "Debe fallar porque el usuario no existe");
        assert_eq!(res.unwrap_err(), "No se encontro el usuario.");
    }

    #[ink::test]
    fn modificar_rol_exitoso() {
        let mut contrato = PrimerContrato::new();
        let acc = account(11);

        assert!(contrato.priv_agregar_usuario_sistema(
            acc,
            "Mario".to_string(),
            "Rossi".to_string(),
            "Calle Uno".to_string(),
            "mario@mail.com".to_string(),
            Rol::Comp,
        ).is_ok());

        let res = contrato.priv_modificar_rol(acc, Rol::Vend);
        assert!(res.is_ok(), "El cambio de rol debería ser exitoso");

        let u = contrato.usuarios.get(acc).unwrap();
        assert_eq!(u.rol, Rol::Vend, "El rol debe haberse actualizado a Vend");
    }

    #[ink::test]
    fn modificar_rol_invalido_se_rechaza() {
        let mut contrato = PrimerContrato::new();
        let acc = account(12);

        assert!(contrato.priv_agregar_usuario_sistema(
            acc,
            "Lucía".to_string(),
            "Fernández".to_string(),
            "Calle Dos".to_string(),
            "lucia@mail.com".to_string(),
            Rol::Comp,
        ).is_ok());

        let res = contrato.priv_modificar_rol(acc, Rol::Comp); 
        if res.is_err() {
            assert_eq!(res.unwrap_err(), "El usuario ya posee ese rol.");
        } else {
            let u = contrato.usuarios.get(acc).unwrap();
            assert_eq!(u.rol, Rol::Comp);
        }
    }

    #[ink::test]
    fn cargar_producto_precio_cero_falla() {
        let mut contrato = PrimerContrato::new();
        let acc = account(20);

        contrato.priv_agregar_usuario_sistema(
            acc,
            "Carlos".to_string(),
            "García".to_string(),
            "Dir".to_string(),
            "carlos@mail.com".to_string(),
            Rol::Vend,
        ).unwrap();

        let res = contrato.priv_cargar_producto(
            acc,
            "ProductoX".to_string(),
            "Desc".to_string(),
            0, 
            "Categoria".to_string(),
            10,
        );

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "Precio no valido");
    }

    #[ink::test]
    fn cargar_producto_stock_cero_falla() {
        let mut contrato = PrimerContrato::new();
        let acc = account(21);

        contrato.priv_agregar_usuario_sistema(
            acc,
            "Marta".to_string(),
            "Lopez".to_string(),
            "Dir".to_string(),
            "marta@mail.com".to_string(),
            Rol::Vend,
        ).unwrap();

        let res = contrato.priv_cargar_producto(
            acc,
            "ProductoY".to_string(),
            "Desc".to_string(),
            100,
            "Categoria".to_string(),
            0, 
        );

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "Stock no valido");
    }

    #[ink::test]
    fn cargar_producto_usuario_inexistente_falla() {
        let mut contrato = PrimerContrato::new();
        let acc = account(22);

        let res = contrato.priv_cargar_producto(
            acc,
            "Prod".to_string(),
            "Desc".to_string(),
            100,
            "Categoria".to_string(),
            5,
        );

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "No se encontro el usuario.");
    }

    #[ink::test]
    fn cargar_producto_usuario_no_vendedor_falla() {
        let mut contrato = PrimerContrato::new();
        let acc = account(23);

        contrato.priv_agregar_usuario_sistema(
            acc,
            "Pepe".to_string(),
            "Suarez".to_string(),
            "Dir".to_string(),
            "pepe@mail.com".to_string(),
            Rol::Comp, 
        ).unwrap();

        let res = contrato.priv_cargar_producto(
            acc,
            "Prod".to_string(),
            "Desc".to_string(),
            200,
            "Categoria".to_string(),
            5,
        );

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "El usuario no tiene permisos para cargar productos. No es vendedor.");
    }

    #[ink::test]
    fn cargar_producto_exitoso_vendedor() {
        let mut contrato = PrimerContrato::new();
        let acc = account(24);

        contrato.priv_agregar_usuario_sistema(
            acc,
            "Vero".to_string(),
            "Alvarez".to_string(),
            "Dir".to_string(),
            "vero@mail.com".to_string(),
            Rol::Vend,
        ).unwrap();

        let res = contrato.priv_cargar_producto(
            acc,
            "ProductoZ".to_string(),
            "Descripcion Z".to_string(),
            500,
            "Categoria Z".to_string(),
            15,
        );

        assert!(res.is_ok(), "El producto debería cargarse correctamente");

        assert_eq!(contrato.dimension_logica_productos, 1);

        let almacenado = contrato.historial_productos.get(1).expect("No se guardó el producto");
        assert_eq!(almacenado.1, 15, "Stock debe coincidir con el cargado");
    }

    #[ink::test]
    fn visualizar_productos_usuario_inexistente_falla() {
        let contrato = PrimerContrato::new();
        let acc = account(30);

        let res = contrato.priv_visualizar_productos_propios(acc);

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "No se encontro el usuario.");
    }

    #[ink::test]
    fn visualizar_productos_usuario_comprador_falla() {
        let mut contrato = PrimerContrato::new();
        let acc = account(31);

        contrato.priv_agregar_usuario_sistema(
            acc,
            "Juan".to_string(),
            "Comprador".to_string(),
            "Dir".to_string(),
            "juan@mail.com".to_string(),
            Rol::Comp,
        ).unwrap();

        let res = contrato.priv_visualizar_productos_propios(acc);

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "El usuario es comprador.");
    }

    #[ink::test]
    fn visualizar_productos_vendedor_sin_productos_falla() {
        let mut contrato = PrimerContrato::new();
        let acc = account(32);

        contrato.priv_agregar_usuario_sistema(
            acc,
            "Ana".to_string(),
            "Vendedora".to_string(),
            "Dir".to_string(),
            "ana@mail.com".to_string(),
            Rol::Vend,
        ).unwrap();

        let res = contrato.priv_visualizar_productos_propios(acc);

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "El vendedor no tiene productos.");
    }

    #[ink::test]
    fn visualizar_productos_vendedor_con_productos() {
        let mut contrato = PrimerContrato::new();
        let acc = account(33);

        contrato.priv_agregar_usuario_sistema(
            acc,
            "Pedro".to_string(),
            "Vendedor".to_string(),
            "Dir".to_string(),
            "pedro@mail.com".to_string(),
            Rol::Vend,
        ).unwrap();

        contrato.priv_cargar_producto(
            acc,
            "Producto1".to_string(),
            "Desc".to_string(),
            100,
            "Categoria".to_string(),
            5,
        ).unwrap();

        let res = contrato.priv_visualizar_productos_propios(acc);

        assert!(res.is_ok());
        let lista = res.unwrap();
        assert_eq!(lista.len(), 1);
        assert_eq!(lista[0].1, 5);
    }

    #[ink::test]
    fn visualizar_productos_ambos_con_productos() {
        let mut contrato = PrimerContrato::new();
        let acc = account(34);

        contrato.priv_agregar_usuario_sistema(
            acc,
            "Lucia".to_string(),
            "Mixta".to_string(),
            "Dir".to_string(),
            "lucia@mail.com".to_string(),
            Rol::Ambos,
        ).unwrap();

        contrato.priv_cargar_producto(
            acc,
            "Producto2".to_string(),
            "Desc".to_string(),
            150,
            "Categoria".to_string(),
            8,
        ).unwrap();

        let res = contrato.priv_visualizar_productos_propios(acc);

        assert!(res.is_ok());
        let lista = res.unwrap();
        assert_eq!(lista.len(), 1);
        assert_eq!(lista[0].1, 8);
    }

    #[ink::test]
    fn crear_publicacion_usuario_inexistente_falla() {
        let mut contrato = PrimerContrato::new();
        let acc = account(40);

        let res = contrato.priv_crear_publicacion(acc, vec![(1, 2)]);

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "No se encontro el usuario.");
    }

    #[ink::test]
    fn crear_publicacion_producto_cantidad_cero_falla() {
        let mut contrato = PrimerContrato::new();
        let acc = account(41);

        contrato.priv_agregar_usuario_sistema(
            acc,
            "Luis".to_string(),
            "Vendedor".to_string(),
            "Dir".to_string(),
            "luis@mail.com".to_string(),
            Rol::Vend,
        ).unwrap();

        contrato.priv_cargar_producto(
            acc,
            "Prod".to_string(),
            "Desc".to_string(),
            100,
            "Cat".to_string(),
            10,
        ).unwrap();

        let res = contrato.priv_crear_publicacion(acc, vec![(1, 0)]);

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "Un producto tiene cantidades no validas.");
    }

    #[ink::test]
    fn crear_publicacion_producto_no_pertenece_falla() {
        let mut contrato = PrimerContrato::new();
        let acc1 = account(42);
        let acc2 = account(43);

        contrato.priv_agregar_usuario_sistema(
            acc1,
            "Ana".to_string(),
            "Vend1".to_string(),
            "Dir".to_string(),
            "ana@mail.com".to_string(),
            Rol::Vend,
        ).unwrap();

        contrato.priv_cargar_producto(
            acc1,
            "Prod".to_string(),
            "Desc".to_string(),
            100,
            "Cat".to_string(),
            5,
        ).unwrap();

        contrato.priv_agregar_usuario_sistema(
            acc2,
            "Beto".to_string(),
            "Vend2".to_string(),
            "Dir".to_string(),
            "beto@mail.com".to_string(),
            Rol::Vend,
        ).unwrap();

        let res = contrato.priv_crear_publicacion(acc2, vec![(1, 2)]);

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "El vendedor no posee ese producto.");
    }

    #[ink::test]
    fn crear_publicacion_sin_stock_falla() {
        let mut contrato = PrimerContrato::new();
        let acc = account(44);

        contrato.priv_agregar_usuario_sistema(
            acc,
            "Clara".to_string(),
            "Vend".to_string(),
            "Dir".to_string(),
            "clara@mail.com".to_string(),
            Rol::Vend,
        ).unwrap();

        contrato.priv_cargar_producto(
            acc,
            "Prod".to_string(),
            "Desc".to_string(),
            50,
            "Cat".to_string(),
            3, 
        ).unwrap();

        let res = contrato.priv_crear_publicacion(acc, vec![(1, 10)]);

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "No hay stock suficiente.");
    }

    #[ink::test]
    fn crear_publicacion_exitosa() {
        let mut contrato = PrimerContrato::new();
        let acc = account(45);

        contrato.priv_agregar_usuario_sistema(
            acc,
            "Diego".to_string(),
            "Vend".to_string(),
            "Dir".to_string(),
            "diego@mail.com".to_string(),
            Rol::Vend,
        ).unwrap();

        contrato.priv_cargar_producto(
            acc,
            "Prod".to_string(),
            "Desc".to_string(),
            200,
            "Cat".to_string(),
            5,
        ).unwrap();

        let res = contrato.priv_crear_publicacion(acc, vec![(1, 2)]);
        assert!(res.is_ok());

        assert_eq!(contrato.historial_publicaciones.len(), 1);

        let prod = contrato.historial_productos.get(1).unwrap();
        assert_eq!(prod.1, 3); //Comprobar si se descuenta el stock del producto almacenado al crear la publicación
    }

    #[ink::test]
    fn visualizar_publicacion_inexistente_falla() {
        let contrato = PrimerContrato::new();

        let res = contrato.priv_visualizar_productos_de_publicacion(1);

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "No se encontro la publicacion.");
    }

    #[ink::test]
    fn visualizar_publicacion_existente_ok() {
        let mut contrato = PrimerContrato::new();
        let acc = account(50);

        contrato.priv_agregar_usuario_sistema(
            acc,
            "Laura".to_string(),
            "Vendedora".to_string(),
            "Dir".to_string(),
            "laura@mail.com".to_string(),
            Rol::Vend,
        ).unwrap();

        contrato.priv_cargar_producto(
            acc,
            "Prod".to_string(),
            "Desc".to_string(),
            100,
            "Cat".to_string(),
            10,
        ).unwrap();

        contrato.priv_crear_publicacion(acc, vec![(1, 2)]).unwrap();

        let res = contrato.priv_visualizar_productos_de_publicacion(0);
        assert!(res.is_ok());

        let publicacion = res.unwrap();
        assert_eq!(publicacion.id_vendedor, acc);
        assert_eq!(publicacion.productos.len(), 1);
        assert_eq!(publicacion.productos[0].1, 2); 
    }

    #[ink::test]
    fn visualizar_publicacion_id_inexistente_falla() {
        let mut contrato = PrimerContrato::new();
        let acc = account(51);

        contrato.priv_agregar_usuario_sistema(
            acc,
            "Carlos".to_string(),
            "Vend".to_string(),
            "Dir".to_string(),
            "carlos@mail.com".to_string(),
            Rol::Vend,
        ).unwrap();

        contrato.priv_cargar_producto(
            acc,
            "Prod".to_string(),
            "Desc".to_string(),
            50,
            "Cat".to_string(),
            5,
        ).unwrap();

        contrato.priv_crear_publicacion(acc, vec![(1, 1)]).unwrap();

        let res = contrato.priv_visualizar_productos_de_publicacion(99);

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "No se encontro la publicacion.");
    }

   #[ink::test]
    fn crear_orden_exito() {
        let mut contrato = PrimerContrato::new();

        let vendedor = account(60);
        let comprador = account(61);

        contrato.priv_agregar_usuario_sistema(
            vendedor,
            "Vendedor".to_string(),
            "V".to_string(),
            "Dir".to_string(),
            "v@mail.com".to_string(),
            Rol::Vend,
        ).unwrap();

        contrato.priv_agregar_usuario_sistema(
            comprador,
            "Comprador".to_string(),
            "C".to_string(),
            "Dir".to_string(),
            "c@mail.com".to_string(),
            Rol::Comp,
        ).unwrap();

        contrato.priv_cargar_producto(
            vendedor,
            "Producto".to_string(),
            "Desc".to_string(),
            100,
            "Cat".to_string(),
            10,
        ).unwrap();

        contrato.priv_crear_publicacion(vendedor, vec![(1, 2)]).unwrap(); // ID de la publicación = 0;  

        let resultado = contrato.priv_crear_orden_de_compra(comprador, 0);
        assert!(resultado.is_ok());

        let historial = contrato.historial_ordenes_de_compra.len();
        assert_eq!(historial, 1);
    }

    #[ink::test]
    fn crear_orden_propia_falla() {
        let mut contrato = PrimerContrato::new();
        let usuario = account(62);

        contrato.priv_agregar_usuario_sistema(
            usuario,
            "Mixto".to_string(),
            "U".to_string(),
            "Dir".to_string(),
            "m@mail.com".to_string(),
            Rol::Ambos,
        ).unwrap();

        contrato.priv_cargar_producto(
            usuario,
            "Prod".to_string(),
            "Desc".to_string(),
            50,
            "Cat".to_string(),
            5,
        ).unwrap();

        contrato.priv_crear_publicacion(usuario, vec![(1, 1)]).unwrap(); 

        let resultado = contrato.priv_crear_orden_de_compra(usuario, 0);
        assert!(resultado.is_err());
        assert_eq!(resultado.unwrap_err(), "El usuario no puede comprar sus propias publicaciones.");
    }

    #[ink::test]
    fn crear_orden_publicacion_comp_falla() {
        let mut contrato = PrimerContrato::new();
        let vendedor = account(63);
        let comprador = account(64);

        contrato.priv_agregar_usuario_sistema(
            vendedor,
            "V".to_string(),
            "V".to_string(),
            "Dir".to_string(),
            "v@mail.com".to_string(),
            Rol::Vend,
        ).unwrap();

        contrato.priv_agregar_usuario_sistema(
            comprador,
            "C".to_string(),
            "C".to_string(),
            "Dir".to_string(),
            "c@mail.com".to_string(),
            Rol::Comp,
        ).unwrap();

        contrato.priv_cargar_producto(
            vendedor,
            "Prod".to_string(),
            "Desc".to_string(),
            30,
            "Cat".to_string(),
            5,
        ).unwrap();

        contrato.priv_crear_publicacion(vendedor, vec![(1, 1)]).unwrap(); 

        contrato.priv_modificar_rol(vendedor, Rol::Comp).unwrap();

        let resultado = contrato.priv_crear_orden_de_compra(comprador, 0);
        assert!(resultado.is_err()); //El vendedor de la publicación cambió su rol a comprador. Debe anular sus publicaciones;
        assert_eq!(resultado.unwrap_err(), "La publicacion ya no se encuentra disponible.");
    }

    #[ink::test]
    fn crear_orden_sin_stock_falla() {
        let mut contrato = PrimerContrato::new();
        let vendedor = account(65);
        let comprador = account(66);

        contrato.priv_agregar_usuario_sistema(
            vendedor,
            "V".to_string(),
            "V".to_string(),
            "Dir".to_string(),
            "v@mail.com".to_string(),
            Rol::Vend,
        ).unwrap();

        contrato.priv_agregar_usuario_sistema(
            comprador,
            "C".to_string(),
            "C".to_string(),
            "Dir".to_string(),
            "c@mail.com".to_string(),
            Rol::Comp,
        ).unwrap();

        contrato.priv_cargar_producto(
            vendedor,
            "Prod".to_string(),
            "Desc".to_string(),
            20,
            "Cat".to_string(),
            1,
        ).unwrap();

        contrato.priv_crear_publicacion(vendedor, vec![(1, 1)]).unwrap(); 

        let mut publicacion = contrato.priv_visualizar_productos_de_publicacion(0).unwrap();
        publicacion.disponible = false; //Se fuerza la disponibilidad de la publicación en falso;
        let pos = contrato.devolver_posicion_publicacion(0).unwrap();
        contrato.historial_publicaciones.set(pos, &(0u32, publicacion)); //Se actualiza la publicación;

        let resultado = contrato.priv_crear_orden_de_compra(comprador, 0);
        assert!(resultado.is_err());
        assert_eq!(resultado.unwrap_err(), "La publicacion ya no tiene stock");
    }

    #[ink::test]
    fn crear_orden_usuario_inexistente_falla() {
        let mut contrato = PrimerContrato::new();
        let vendedor = account(67);

        contrato.priv_agregar_usuario_sistema(
            vendedor,
            "V".to_string(),
            "V".to_string(),
            "Dir".to_string(),
            "v@mail.com".to_string(),
            Rol::Vend,
        ).unwrap();

        contrato.priv_cargar_producto(
            vendedor,
            "Prod".to_string(),
            "Desc".to_string(),
            15,
            "Cat".to_string(),
            2,
        ).unwrap();

        contrato.priv_crear_publicacion(vendedor, vec![(1, 1)]).unwrap();

        let account_invalido: AccountId = [0x0; 32].into();

        let resultado = contrato.priv_crear_orden_de_compra(account_invalido, 0);
        assert!(resultado.is_err());
    }

    #[ink::test]
    fn crear_orden_publicacion_inexistente_falla() {
        let mut contrato = PrimerContrato::new();
        let comprador = account(68);

        contrato.priv_agregar_usuario_sistema(
            comprador,
            "C".to_string(),
            "C".to_string(),
            "Dir".to_string(),
            "c@mail.com".to_string(),
            Rol::Comp,
        ).unwrap();

        let id_pub_invalido = 999u32;
        let resultado = contrato.priv_crear_orden_de_compra(comprador, id_pub_invalido);
        assert!(resultado.is_err());
    }


    #[ink::test]
    fn cancelar_compra_comprador_exito() {
        let mut contrato = PrimerContrato::new();
        let vendedor = account(100);
        let comprador = account(101);

        contrato.priv_agregar_usuario_sistema(
            vendedor, "V".into(), "V".into(), "Dir".into(), "v@mail".into(), Rol::Vend
        ).unwrap();
        contrato.priv_agregar_usuario_sistema(
            comprador, "C".into(), "C".into(), "Dir".into(), "c@mail".into(), Rol::Comp
        ).unwrap();

        contrato.priv_cargar_producto(vendedor, "P".into(), "D".into(), 10, "cat".into(), 5).unwrap();
        contrato.priv_crear_publicacion(vendedor, vec![(1, 2)]).unwrap(); // pub 0

        contrato.priv_crear_orden_de_compra(comprador, 0).unwrap();

        let res = contrato.priv_cancelar_compra(comprador, 0);
        assert!(res.is_ok());

        let (_, orden) = contrato.historial_ordenes_de_compra.get(0).unwrap();
        assert!(orden.cancelacion.1, "El comprador debe haber marcado la cancelación");
        assert!(!orden.cancelacion.0, "El vendedor aún no debe haber marcado la cancelación");
        assert_eq!(orden.estado, EstadoCompra::Pendiente, "El estado permanece Pendiente hasta que ambas partes cancelen");
    }

    #[ink::test]
    fn cancelar_compra_vendedor_exito() {
        let mut contrato = PrimerContrato::new();
        let vendedor = account(102);
        let comprador = account(103);

        contrato.priv_agregar_usuario_sistema(
            vendedor, "V".into(), "V".into(), "Dir".into(), "v@mail".into(), Rol::Vend
        ).unwrap();
        contrato.priv_agregar_usuario_sistema(
            comprador, "C".into(), "C".into(), "Dir".into(), "c@mail".into(), Rol::Comp
        ).unwrap();

        contrato.priv_cargar_producto(vendedor, "P".into(), "D".into(), 10, "cat".into(), 5).unwrap(); //5 productos alcemanados;
        contrato.priv_crear_publicacion(vendedor, vec![(1, 2)]).unwrap(); //3 productos almcenados;

        contrato.priv_crear_orden_de_compra(comprador, 0).unwrap(); //1 producto almacenado. (Restockeo automatico de la publicacion);

        contrato.priv_cancelar_compra(comprador, 0).unwrap();

        let res = contrato.priv_cancelar_compra(vendedor, 0); //3 productos almacenados;
        assert!(res.is_ok());

        let (_, orden) = contrato.historial_ordenes_de_compra.get(0).unwrap();
        assert!(orden.cancelacion.0 && orden.cancelacion.1, "Ambas partes deben haber marcado cancelación");

        let (_, stock) = contrato.historial_productos.get(1).unwrap();
        assert_eq!(stock, 3); //El stock que estaba en la publicación debe restaurse correctamente; (5 - 2 - 2 + 2) = 3; 
    }

    #[ink::test]
    fn cancelar_compra_orden_no_existe_falla() {
        let mut contrato = PrimerContrato::new();
        let acc = account(104);

        contrato.priv_agregar_usuario_sistema(
            acc, "U".into(), "U".into(), "Dir".into(), "u@mail".into(), Rol::Vend
        ).unwrap();

        let id_orden_invalido = 999u32;
        let res = contrato.priv_cancelar_compra(acc, id_orden_invalido);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "No se encontro la orden.");
    }

    #[ink::test]
    fn cancelar_compra_estado_no_cancelable_falla() {
        let mut contrato = PrimerContrato::new();
        let vendedor = account(105);
        let comprador = account(106);

        contrato.priv_agregar_usuario_sistema(vendedor, "V".into(), "V".into(), "D".into(), "v@mail".into(), Rol::Vend).unwrap();
        contrato.priv_agregar_usuario_sistema(comprador, "C".into(), "C".into(), "D".into(), "c@mail".into(), Rol::Comp).unwrap();
        contrato.priv_cargar_producto(vendedor, "P".into(), "D".into(), 10, "cat".into(), 3).unwrap();
        contrato.priv_crear_publicacion(vendedor, vec![(1,1)]).unwrap();
        contrato.priv_crear_orden_de_compra(comprador, 0).unwrap();

        let (pos_id, mut orden) = contrato.historial_ordenes_de_compra.get(0).unwrap();
        orden.estado = EstadoCompra::Enviado; //Se fuerza el estado de la orden a Enviado;
        contrato.historial_ordenes_de_compra.set(0, &(pos_id, orden));

        let res = contrato.priv_cancelar_compra(comprador, 0);
        assert!(res.is_err()); //La orden tiene el estado "Enviado";
        assert_eq!(res.unwrap_err(), "Ya no se puede cancelar la compra.");
    }

    #[ink::test]
    fn cancelar_compra_usuario_inexistente_falla() {
        let mut contrato = PrimerContrato::new();
        let id_orden = 0u32;
        let account_invalido: AccountId = [0x0; 32].into();

        let res = contrato.priv_cancelar_compra(account_invalido, id_orden);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "No se encontro el usuario.");
    }

    #[ink::test]
     fn priv_recibir_compra_exito() {
        let mut contrato = PrimerContrato::new();
        let vendedor = account(200);
        let comprador = account(201);

        contrato.priv_agregar_usuario_sistema(
            vendedor, "V".into(), "V".into(), "Dir".into(), "v@mail".into(), Rol::Vend
        ).unwrap();
        contrato.priv_agregar_usuario_sistema(
            comprador, "C".into(), "C".into(), "Dir".into(), "c@mail".into(), Rol::Comp
        ).unwrap();

        contrato.priv_cargar_producto(vendedor, "P".into(), "D".into(), 10, "cat".into(), 3).unwrap();
        contrato.priv_crear_publicacion(vendedor, vec![(1,1)]).unwrap();
        contrato.priv_crear_orden_de_compra(comprador, 0).unwrap();

        contrato.priv_enviar_compra(vendedor, 0).unwrap();

        let res = contrato.priv_recibir_compra(comprador, 0);
        assert!(res.is_ok());

        let (_, orden) = contrato.historial_ordenes_de_compra.get(0).unwrap();
        assert_eq!(orden.estado, EstadoCompra::Recibido);
    }

    #[ink::test]
    fn priv_recibir_compra_usuario_no_encontrado_falla() {
        let mut contrato = PrimerContrato::new();
        let account_invalido: AccountId = [0x0; 32].into();
        let res = contrato.priv_recibir_compra(account_invalido, 0);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "No se encontro el usuario.");
    }

    #[ink::test]
    fn priv_recibir_compra_orden_no_encontrada_falla() {
        let mut contrato = PrimerContrato::new();
        let comprador = account(210);
        contrato.priv_agregar_usuario_sistema(
            comprador, "C".into(), "C".into(), "Dir".into(), "c@mail".into(), Rol::Comp
        ).unwrap();

        let res = contrato.priv_recibir_compra(comprador, 999);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "No existe la orden buscada.");
    }

    #[ink::test]
    fn priv_recibir_compra_no_enviado_falla() {
        let mut contrato = PrimerContrato::new();
        let vendedor = account(220);
        let comprador = account(221);

        contrato.priv_agregar_usuario_sistema(
            vendedor, "V".into(), "V".into(), "Dir".into(), "v@mail".into(), Rol::Vend
        ).unwrap();
        contrato.priv_agregar_usuario_sistema(
            comprador, "C".into(), "C".into(), "Dir".into(), "c@mail".into(), Rol::Comp
        ).unwrap();

        contrato.priv_cargar_producto(vendedor, "P".into(), "D".into(), 10, "cat".into(), 3).unwrap();
        contrato.priv_crear_publicacion(vendedor, vec![(1,1)]).unwrap();
        contrato.priv_crear_orden_de_compra(comprador, 0).unwrap();

        let res = contrato.priv_recibir_compra(comprador, 0);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "El producto todavia no fue enviado.");
    }

    #[ink::test]
    fn priv_recibir_compra_usuario_no_comprador_falla() {
        let mut contrato = PrimerContrato::new();
        let vendedor = account(230);
        let comprador = account(231);
        let otro = account(232); 

        contrato.priv_agregar_usuario_sistema(
            vendedor, "V".into(), "V".into(), "Dir".into(), "v@mail".into(), Rol::Vend
        ).unwrap();
        contrato.priv_agregar_usuario_sistema(
            comprador, "C".into(), "C".into(), "Dir".into(), "c@mail".into(), Rol::Comp
        ).unwrap();
        contrato.priv_agregar_usuario_sistema(
            otro, "O".into(), "O".into(), "Dir".into(), "o@mail".into(), Rol::Comp
        ).unwrap();

        contrato.priv_cargar_producto(vendedor, "P".into(), "D".into(), 10, "cat".into(), 3).unwrap();
        contrato.priv_crear_publicacion(vendedor, vec![(1,1)]).unwrap();
        contrato.priv_crear_orden_de_compra(comprador, 0).unwrap();
        contrato.priv_enviar_compra(vendedor, 0).unwrap();

        let res = contrato.priv_recibir_compra(otro, 0);
        assert!(res.is_err()); //Otro usuario externo a la orden de la compra en cuestión no puede alterarla;
        assert_eq!(res.unwrap_err(), "No se encontro la orden de compra.");
    }

    #[ink::test]
    fn priv_calificar_exito() {
        let mut contrato = PrimerContrato::new();
        let vendedor = account(150);
        let comprador = account(151);

        contrato.priv_agregar_usuario_sistema(
            vendedor, "V".into(), "V".into(), "Dir".into(), "v@mail".into(), Rol::Vend
        ).unwrap();
        contrato.priv_agregar_usuario_sistema(
            comprador, "C".into(), "C".into(), "Dir".into(), "c@mail".into(), Rol::Comp
        ).unwrap();

        contrato.priv_cargar_producto(vendedor, "P".into(), "D".into(), 10, "cat".into(), 5).unwrap();
        contrato.priv_crear_publicacion(vendedor, vec![(1,1)]).unwrap();
        contrato.priv_crear_orden_de_compra(comprador, 0).unwrap();
        contrato.priv_enviar_compra(vendedor, 0).unwrap();
        contrato.priv_recibir_compra(comprador, 0).unwrap();

        let res = contrato.priv_calificar(0, 5, comprador);
        assert!(res.is_ok());

        let vend = contrato.buscar_usuario(vendedor).unwrap();
        assert_eq!(vend.datos_vendedor.unwrap().reputacion_como_vendedor[0], 5);
    }

    #[ink::test]
    fn priv_calificar_valor_fuera_rango_falla() {
        let mut contrato = PrimerContrato::new();
        let vendedor = account(152);
        let comprador = account(153);

        contrato.priv_agregar_usuario_sistema(
            vendedor, "V".into(), "V".into(), "Dir".into(), "v@mail".into(), Rol::Vend
        ).unwrap();
        contrato.priv_agregar_usuario_sistema(
            comprador, "C".into(), "C".into(), "Dir".into(), "c@mail".into(), Rol::Comp
        ).unwrap();

        contrato.priv_cargar_producto(vendedor, "P".into(), "D".into(), 10, "cat".into(), 5).unwrap();
        contrato.priv_crear_publicacion(vendedor, vec![(1,1)]).unwrap();
        contrato.priv_crear_orden_de_compra(comprador, 0).unwrap();
        contrato.priv_enviar_compra(vendedor, 0).unwrap();
        contrato.priv_recibir_compra(comprador, 0).unwrap();

        let res = contrato.priv_calificar(0, 0, comprador);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "El valor de la calificacion no es valido (1..5).");
    }

    #[ink::test]
    fn priv_calificar_duplicada_falla() {
        let mut contrato = PrimerContrato::new();
        let vendedor = account(154);
        let comprador = account(155);

        contrato.priv_agregar_usuario_sistema(
            vendedor, "V".into(), "V".into(), "Dir".into(), "v@mail".into(), Rol::Vend
        ).unwrap();
        contrato.priv_agregar_usuario_sistema(
            comprador, "C".into(), "C".into(), "Dir".into(), "c@mail".into(), Rol::Comp
        ).unwrap();

        contrato.priv_cargar_producto(vendedor, "P".into(), "D".into(), 10, "cat".into(), 5).unwrap();
        contrato.priv_crear_publicacion(vendedor, vec![(1,1)]).unwrap();
        contrato.priv_crear_orden_de_compra(comprador, 0).unwrap();
        contrato.priv_enviar_compra(vendedor, 0).unwrap();
        contrato.priv_recibir_compra(comprador, 0).unwrap();

        contrato.priv_calificar(0, 5, comprador).unwrap();
        let res2 = contrato.priv_calificar(0, 4, comprador);
        assert!(res2.is_err());
        assert_eq!(res2.unwrap_err(), "El usuario ya califico.");
    }

    #[ink::test]
    fn priv_calificar_orden_no_recibida_falla() {
        let mut contrato = PrimerContrato::new();
        let vendedor = account(156);
        let comprador = account(157);

        contrato.priv_agregar_usuario_sistema(
            vendedor, "V".into(), "V".into(), "Dir".into(), "v@mail".into(), Rol::Vend
        ).unwrap();
        contrato.priv_agregar_usuario_sistema(
            comprador, "C".into(), "C".into(), "Dir".into(), "c@mail".into(), Rol::Comp
        ).unwrap();

        contrato.priv_cargar_producto(vendedor, "P".into(), "D".into(), 10, "cat".into(), 5).unwrap();
        contrato.priv_crear_publicacion(vendedor, vec![(1,1)]).unwrap();
        contrato.priv_crear_orden_de_compra(comprador, 0).unwrap();

        let res = contrato.priv_calificar(0, 5, comprador);
        assert!(res.is_err());
    }

    #[ink::test]
    fn priv_calificar_usuario_no_encontrado_falla() {
        let mut contrato = PrimerContrato::new();
        let vendedor = account(158);
        let comprador = account(159);

        contrato.priv_agregar_usuario_sistema(
            vendedor, "V".into(), "V".into(), "Dir".into(), "v@mail".into(), Rol::Vend
        ).unwrap();

        contrato.priv_cargar_producto(vendedor, "P".into(), "D".into(), 10, "cat".into(), 5).unwrap();
        contrato.priv_crear_publicacion(vendedor, vec![(1,1)]).unwrap();

        let account_invalido: AccountId = [0x0; 32].into();
        let res = contrato.priv_calificar(0, 5, account_invalido);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "No se encontro el usuario.");
    }

    #[ink::test]
    fn priv_calificar_usuario_no_participante_falla() {
        let mut contrato = PrimerContrato::new();
        let vendedor = account(160);
        let comprador = account(161);
        let otro = account(162);

        contrato.priv_agregar_usuario_sistema(
            vendedor, "V".into(), "V".into(), "Dir".into(), "v@mail".into(), Rol::Vend
        ).unwrap();
        contrato.priv_agregar_usuario_sistema(
            comprador, "C".into(), "C".into(), "Dir".into(), "c@mail".into(), Rol::Comp
        ).unwrap();
        contrato.priv_agregar_usuario_sistema(
            otro, "O".into(), "O".into(), "Dir".into(), "o@mail".into(), Rol::Comp
        ).unwrap();

        contrato.priv_cargar_producto(vendedor, "P".into(), "D".into(), 10, "cat".into(), 5).unwrap();
        contrato.priv_crear_publicacion(vendedor, vec![(1,1)]).unwrap();
        contrato.priv_crear_orden_de_compra(comprador, 0).unwrap();
        contrato.priv_enviar_compra(vendedor, 0).unwrap();
        contrato.priv_recibir_compra(comprador, 0).unwrap();

        let res = contrato.priv_calificar(0, 4, otro);
        assert!(res.is_err());
    }


    #[ink::test]
    fn modificar_a_rol_ambos_sin_datos_previos() {
        let mut contrato = PrimerContrato::new();
        let usuario = account(200);

        contrato.priv_agregar_usuario_sistema(
            usuario,
            "Nombre".to_string(),
            "Apellido".to_string(),
            "Dir".to_string(),
            "u@mail.com".to_string(),
            Rol::Comp,
        ).unwrap();

        let res = contrato.priv_modificar_rol(usuario, Rol::Ambos);
        assert!(res.is_ok());

        let u = contrato.buscar_usuario(usuario).unwrap();
        assert!(u.datos_comprador.is_some());
        assert!(u.datos_vendedor.is_some());

        assert_eq!(u.datos_comprador.as_ref().unwrap().ordenes_de_compra.len(), 0);
        assert_eq!(u.datos_comprador.as_ref().unwrap().reputacion_como_comprador.len(), 0);
        assert_eq!(u.datos_vendedor.as_ref().unwrap().productos.len(), 0);
        assert_eq!(u.datos_vendedor.as_ref().unwrap().publicaciones.len(), 0);
        assert_eq!(u.datos_vendedor.as_ref().unwrap().reputacion_como_vendedor.len(), 0);
    }

    #[ink::test]
    fn modificar_a_rol_ambos_con_datos_comprador_previos() {
        let mut contrato = PrimerContrato::new();
        let usuario = account(201);

        contrato.priv_agregar_usuario_sistema(
            usuario,
            "Comprador".to_string(),
            "Uno".to_string(),
            "Dir".to_string(),
            "c@mail.com".to_string(),
            Rol::Comp,
        ).unwrap();

        //Los datos del comprador no deben perderse si se modifica su rol;
        {
            let mut u = contrato.buscar_usuario(usuario).unwrap();
            u.datos_comprador = Some(Comprador {
                ordenes_de_compra: vec![1, 2],
                reputacion_como_comprador: vec![5],
            });
            contrato.usuarios.insert(usuario, &u);
        }

        let res = contrato.priv_modificar_rol(usuario, Rol::Ambos);
        assert!(res.is_ok());

        let u = contrato.buscar_usuario(usuario).unwrap();
        assert!(u.datos_comprador.is_some()); //Rol previo
        assert!(u.datos_vendedor.is_some()); //Rol nuevo (ambos)

        //Fijarse que efectivamente sus datos siguen estando
        assert_eq!(u.datos_comprador.as_ref().unwrap().ordenes_de_compra.len(), 2);
        assert_eq!(u.datos_comprador.as_ref().unwrap().reputacion_como_comprador.len(), 1);
    }

    #[ink::test]
    fn modificar_a_rol_ambos_con_datos_vendedor_previos() {
        let mut contrato = PrimerContrato::new();
        let usuario = account(202);

        contrato.priv_agregar_usuario_sistema(
            usuario,
            "Vendedor".to_string(),
            "Dos".to_string(),
            "Dir".to_string(),
            "v@mail.com".to_string(),
            Rol::Vend,
        ).unwrap();

        {
            let mut u = contrato.buscar_usuario(usuario).unwrap();
            u.datos_vendedor = Some(Vendedor {
                productos: vec![10],
                publicaciones: vec![5],
                reputacion_como_vendedor: vec![3],
            });
            contrato.usuarios.insert(usuario, &u);
        }

        let res = contrato.priv_modificar_rol(usuario, Rol::Ambos);
        assert!(res.is_ok());

        let u = contrato.buscar_usuario(usuario).unwrap();
        assert!(u.datos_comprador.is_some());
        assert!(u.datos_vendedor.is_some());

        assert_eq!(u.datos_vendedor.as_ref().unwrap().productos.len(), 1);
        assert_eq!(u.datos_vendedor.as_ref().unwrap().publicaciones.len(), 1);
    }

    #[ink::test]
    fn modificar_a_rol_ambos_con_ambos_datos_existentes() {
        let mut contrato = PrimerContrato::new();
        let usuario = account(203);

        contrato.priv_agregar_usuario_sistema(
            usuario,
            "Ambos".to_string(),
            "Tres".to_string(),
            "Dir".to_string(),
            "a@mail.com".to_string(),
            Rol::Ambos,
        ).unwrap();

        assert!(contrato.priv_modificar_rol(usuario, Rol::Vend).is_ok());
        let res = contrato.priv_modificar_rol(usuario, Rol::Ambos);
        assert!(res.is_ok());

        let u = contrato.buscar_usuario(usuario).unwrap();
        assert!(u.datos_comprador.is_some());
        assert!(u.datos_vendedor.is_some());
    }

    #[ink::test]
    fn enviar_compra_fallo_estado() {
        let mut instance = PrimerContrato::new();
        let vendedor = account(240);
        let comprador = account(241);

        instance.priv_agregar_usuario_sistema(
            vendedor, "V".into(), "V".into(), "Dir".into(), "v@mail".into(), Rol::Vend
        ).unwrap();
        instance.priv_agregar_usuario_sistema(
            comprador, "C".into(), "C".into(), "Dir".into(), "c@mail".into(), Rol::Comp
        ).unwrap();

        instance.priv_cargar_producto(vendedor, "P".into(), "D".into(), 10, "cat".into(), 3).unwrap();
        instance.priv_crear_publicacion(vendedor, vec![(1,1)]).unwrap();

        instance.priv_crear_orden_de_compra(comprador, 0).unwrap();

        let (pos_id, mut orden) = instance.historial_ordenes_de_compra.get(0).unwrap();
        orden.estado = EstadoCompra::Enviado;
        instance.historial_ordenes_de_compra.set(0, &(pos_id, orden));

        let result = instance.priv_enviar_compra(vendedor, 0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "El producto no puede ser enviado.".to_string());
    }
    
        #[ink::test]
        fn enviar_compra_fallo_orden_no_existe() {
            let mut instance = PrimerContrato::new();
            let vendedor = account(240);
            let comprador = account(241);

            instance.priv_agregar_usuario_sistema(
                vendedor, "V".into(), "V".into(), "Dir".into(), "v@mail".into(), Rol::Vend
            ).unwrap();
            instance.priv_agregar_usuario_sistema(
                comprador, "C".into(), "C".into(), "Dir".into(), "c@mail".into(), Rol::Comp
            ).unwrap();

            instance.priv_cargar_producto(vendedor, "P".into(), "D".into(), 10, "cat".into(), 3).unwrap();
            instance.priv_crear_publicacion(vendedor, vec![(1,1)]).unwrap();

            let result = instance.priv_enviar_compra(vendedor, 999); 

            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), "No existe la orden buscada.".to_string());
        }

   
    #[ink::test]
    fn comprobar_rol_ambos_casos() {
        let usuario_ambos = account(1);
        let vendedor = account(2);
        let comprador = account(3);
        let _otro_usuario = account(4);

        let usuario = Usuario::nuevo(
            usuario_ambos,
            "Nombre".into(),
            "Apellido".into(),
            "Dir".into(),
            "u@mail".into(),
            Rol::Ambos,
        );

        let resultado_vendedor = usuario.comprobar_rol(usuario_ambos, comprador);
        assert_eq!(resultado_vendedor, Ok(Rol::Vend));

        let resultado_comprador = usuario.comprobar_rol(vendedor, usuario_ambos);
        assert_eq!(resultado_comprador, Ok(Rol::Comp));

        let resultado_error = usuario.comprobar_rol(vendedor, comprador);
        assert!(resultado_error.is_err());
        assert_eq!(resultado_error.unwrap_err(), "El usuario no participa de la compra.".to_string());
    }


    #[ink::test]
    fn comprobar_rol_errores() {
        let mut instancia = PrimerContrato::new(); 
        let vendedor_real = account(1);
        let comprador_real = account(2);
        let otro_vendedor = account(3);
        let otro_comprador = account(4);

        let mut usuario_vendedor = Usuario::nuevo(
            vendedor_real,
            "Vendedor".to_string(),
            "Apellido".to_string(),
            "Dir".to_string(),
            "v@mail.com".to_string(),
            Rol::Vend,
        );

        let resultado_error_vendedor = usuario_vendedor.comprobar_rol(otro_vendedor, comprador_real);
        assert!(resultado_error_vendedor.is_err());
        assert_eq!(resultado_error_vendedor.unwrap_err(), "El vendedor no posee esta publicacion.".to_string());

        let mut usuario_comprador = Usuario::nuevo(
            comprador_real,
            "Comprador".to_string(),
            "Apellido".to_string(),
            "Dir".to_string(),
            "c@mail.com".to_string(),
            Rol::Comp,
        );

        let resultado_error_comprador = usuario_comprador.comprobar_rol(vendedor_real, otro_comprador);
        assert!(resultado_error_comprador.is_err());
        assert_eq!(resultado_error_comprador.unwrap_err(), "El usuario no posee esa orden de compra.".to_string());
    }

    #[ink::test]
    fn actualizar_publicaciones_exito() {
        let mut contrato = PrimerContrato::new();
        let vendedor = account(210);

        contrato.priv_agregar_usuario_sistema(
            vendedor, "V".into(), "V".into(), "Dir".into(), "v@mail".into(), Rol::Vend
        ).unwrap();

        contrato.priv_cargar_producto(
            vendedor, "Producto".into(), "Desc".into(), 100, "Cat".into(), 5
        ).unwrap();

        contrato.priv_crear_publicacion(vendedor, vec![(1, 2)]).unwrap(); 

        let orig = contrato.priv_visualizar_productos_de_publicacion(0).unwrap();

        let mut nueva = Publicacion {
            id: orig.id,
            productos: orig.productos.clone(),
            precio_final: orig.precio_final + 500,
            id_vendedor: orig.id_vendedor,
            disponible: false,
        };

        assert!(contrato.actualizar_publicaciones(nueva.clone(), 0).is_ok());

        let pos = contrato.devolver_posicion_publicacion(0).unwrap();
        let (_id, guardada) = contrato.historial_publicaciones.get(pos).unwrap();
        assert_eq!(guardada.precio_final, nueva.precio_final);
        assert_eq!(guardada.disponible, nueva.disponible);
    }

    #[ink::test]
    fn actualizar_publicaciones_id_inexistente_falla() {
        let mut contrato = PrimerContrato::new();
        let vendedor = account(211);

        contrato.priv_agregar_usuario_sistema(
            vendedor, "V".into(), "V".into(), "Dir".into(), "v@mail".into(), Rol::Vend
        ).unwrap();

        let pub_falsa = Publicacion {
            id: 999,
            productos: vec![],
            precio_final: 0,
            id_vendedor: vendedor,
            disponible: true,
        };

        let res = contrato.actualizar_publicaciones(pub_falsa, 999);
        assert!(res.is_err());
    }

    #[ink::test]
    fn priv_calificar_comprador_exito() {
        let mut contrato = PrimerContrato::new();
        let comprador = account(250);

        contrato.priv_agregar_usuario_sistema(
            comprador, "Comprador".into(), "C".into(), "Dir".into(), "c@mail".into(), Rol::Comp
        ).unwrap();

        {
            let mut u = contrato.buscar_usuario(comprador).unwrap();
            u.datos_comprador = Some(Comprador {
                ordenes_de_compra: vec![],
                reputacion_como_comprador: vec![3],
            });
            contrato.usuarios.insert(comprador, &u);
        }

        let res = contrato.calificar_comprador(comprador, 5);
        assert!(res.is_ok());

        let u = contrato.buscar_usuario(comprador).unwrap();
        assert_eq!(u.datos_comprador.unwrap().reputacion_como_comprador.last().copied(), Some(5));
    }

    #[ink::test]
    fn priv_calificar_comprador_sin_datos_falla() {
        let mut contrato = PrimerContrato::new();
        let usuario = account(251);

        contrato.priv_agregar_usuario_sistema(
            usuario, "Usuario".into(), "U".into(), "Dir".into(), "u@mail".into(), Rol::Vend
        ).unwrap();

        let res = contrato.calificar_comprador(usuario, 4);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "El comprador no tiene datos cargados.".to_string());
    }

    #[ink::test]
    fn priv_calificar_comprador_usuario_no_encontrado_falla() {
        let mut contrato = PrimerContrato::new();
        let account_invalido: AccountId = [0x0; 32].into();

        let res = contrato.calificar_comprador(account_invalido, 4);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "No se encontro el usuario.".to_string());
    }


     #[ink::test]
    fn es_comprador_ambos_con_rol_comp_ok() {
        let id = AccountId::from([1u8; 32]);
        let usuario = Usuario::nuevo(
            id,
            "Nombre".into(),
            "Apellido".into(),
            "Dir".into(),
            "u@mail".into(),
            Rol::Comp,
        );
        assert!(usuario.es_comprador_ambos().is_ok());
    }

    #[ink::test]
    fn es_comprador_ambos_con_rol_ambos_ok() {
        let id = AccountId::from([2u8; 32]);
        let usuario = Usuario::nuevo(
            id,
            "Nombre".into(),
            "Apellido".into(),
            "Dir".into(),
            "u@mail".into(),
            Rol::Ambos,
        );
        assert!(usuario.es_comprador_ambos().is_ok());
    }

    #[ink::test]
    fn es_comprador_ambos_con_rol_vend_err() {
        let id = AccountId::from([3u8; 32]);
        let usuario = Usuario::nuevo(
            id,
            "Nombre".into(),
            "Apellido".into(),
            "Dir".into(),
            "u@mail".into(),
            Rol::Vend,
        );
        let res = usuario.es_comprador_ambos();
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "El usuario es vendedor.".to_string());
    }


    #[ink::test]
    fn calificar_segun_rol_else_califica_comprador_por_vendedor() {
        let mut contrato = PrimerContrato::new();
        let vendedor = account(240);
        let comprador = account(241);

        contrato.priv_agregar_usuario_sistema(
            vendedor, "V".into(), "V".into(), "Dir".into(), "v@mail".into(), Rol::Vend
        ).unwrap();
        contrato.priv_agregar_usuario_sistema(
            comprador, "C".into(), "C".into(), "Dir".into(), "c@mail".into(), Rol::Comp
        ).unwrap();

        contrato.priv_cargar_producto(vendedor, "P".into(), "D".into(), 10, "cat".into(), 3).unwrap();
        contrato.priv_crear_publicacion(vendedor, vec![(1,1)]).unwrap();
        contrato.priv_crear_orden_de_compra(comprador, 0).unwrap();

        let usuario_vendedor = contrato.buscar_usuario(vendedor).unwrap();
        let (_id, orden) = contrato.historial_ordenes_de_compra.get(0).unwrap();

        contrato.calificar_segun_rol(4u8, orden.clone(), vendedor, comprador, usuario_vendedor).unwrap();

        let (_id2, orden_actualizada) = contrato.historial_ordenes_de_compra.get(0).unwrap();
        assert!(orden_actualizada.calificaciones.1, "La orden debe marcar calificación del comprador por parte del vendedor");

        let comprador_actual = contrato.buscar_usuario(comprador).unwrap();
        assert_eq!(comprador_actual.datos_comprador.unwrap().reputacion_como_comprador.last().copied(), Some(4));
    }
}

}