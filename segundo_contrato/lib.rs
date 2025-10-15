#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod segundo_contrato {

    use primer_contrato::{PrimerContratoRef, Usuario, Rol, EstadoCompra,Comprador,Vendedor, OrdenCompra, Categoria, Producto};
    use ink::storage::Mapping;
    use ink::storage::StorageVec;
    use ink::prelude::vec::Vec;
    use ink::prelude::string::{String, ToString};
    use ink::env::call::FromAccountId;

/// Struct que representa el segundo contrato del sistema.  
/// Este contrato se comunica con el primer contrato mediante su referencia
/// para acceder a la información del marketplace y obtener estadísticas.  
    #[ink(storage)]
    pub struct SegundoContrato {
        marketplace: PrimerContratoRef,
    }
    
    impl SegundoContrato {
        
        #[ink(constructor)]
        /// Constructor del contrato.  
        ///  
        /// Recibe la dirección del contrato principal (PrimerContrato)  
        /// y la guarda como referencia (PrimerContratoRef) para poder interactuar con él.
        pub fn new(primer_contrato_addr: AccountId) -> Self {
            let marketplace = PrimerContratoRef::from_account_id(primer_contrato_addr);
            Self { marketplace }
        }


        #[ink(message)]
        /// Funcion que retorna los cinco vendedores con mejor reputacion promedio.
        pub fn vendedores_mejor_reputacion(&self) -> Result <Vec <AccountId>, String>{
            self.priv_vendedores_mejor_reputacion()
        }
        fn priv_vendedores_mejor_reputacion(&self) -> Result<Vec<AccountId>, String>{
            let vendedores = self.filtrar_vendedores()?;
            let vec_contador: Vec<(AccountId, u8)> = self.contar_promedios_vendedor(vendedores)?;
            
            let top5: Vec<AccountId> = self.calcular_5_mejores(vec_contador)?;

            return Ok(top5);
        }

        #[ink(message)]
        /// Funcion que retorna un vector con la longitud especificada en top, con los productos mas vendidos (id, cantidad).
        ///
        /// Errores posibles: No hay productos vendidos para procesar.
        pub fn productos_mas_vendidos(&self, top: Option<u32>) -> Result<Vec<(u32, u32)>, String>{
            self.priv_productos_mas_vendidos(top)
        } 
        fn priv_productos_mas_vendidos(&self, top: Option<u32>) -> Result<Vec<(u32, u32)>, String>{
            let ordenes = self.marketplace.get_ordenes()?;
            let ordenes = self.filtrar_validas(ordenes);
            if ordenes.is_empty() {
                return Err("No hay productos vendidos de ventas concretadas.".to_string())
            }
            let mut vector_contador: Vec<(u32, u32)> = Vec::new();

            for orden in ordenes{
                self.procesar_orden(&mut vector_contador, orden)?;
            }
            
            vector_contador.sort_by(|a, b| b.1.cmp(&a.1));

            if let Some(cant) = top{
                let top_x = vector_contador.into_iter().take(cant as usize).collect();
                return Ok(top_x);
            } else {
                return Ok(vector_contador);
            }
        }

        #[ink(message)]
        /// Funcion que retorna los cinco compradores con mejor reputacion promedio.
        pub fn compradores_mejor_reputacion(&self) -> Result<Vec<AccountId>, String>{
            self.priv_compradores_mejor_reputacion()
        }
        fn priv_compradores_mejor_reputacion(&self) -> Result<Vec<AccountId>, String>{
            let compradores = self.filtrar_compradores()?;
            let vec_contador: Vec<(AccountId, u8)> = self.contar_promedios_comprador(compradores)?;
            
            let top5: Vec<AccountId> = self.calcular_5_mejores(vec_contador)?;

            return Ok(top5);
        }

        #[ink(message)]
        /// Funcion que retorna un vector de tuplas con el id de los compradores y la cantidad de ordenes de compras realizadas por este. 
        /// En formato (id, cantidad)
        /// Errores posibles: No hay usuarios con datos para procesar
        pub fn cantidad_ordenes_por_usuarios(&self) -> Result<Vec<(AccountId, u32)>, String>{
            self.priv_cantidad_ordenes_por_usuarios()
        }
        fn priv_cantidad_ordenes_por_usuarios(&self) -> Result<Vec<(AccountId, u32)>, String>{
            let usuarios = self.marketplace.get_usuarios()?;
            let usuarios = self.filtrar_con_datos_comprador(usuarios);

            if usuarios.is_empty(){
                return Err("No hay usuarios con datos de comprador cargados en sistema.".to_string());
            }

            let cantidades = self.contar_cantidades(usuarios);

            return Ok(cantidades);
        }

        #[ink(message)]
        /// Funcion que retorna un vector de tuplas, con cada categoria, su cantidad de ventas totales 
        /// y el promedio de calificaciones de cada uno de sus productos vendidos
        pub fn estadisticas_por_categoria(&self) -> Result< Vec< (Categoria, u32, u8)>, String>{
            self.priv_estadisticas_por_categoria()
        }
        fn priv_estadisticas_por_categoria(&self) -> Result< Vec< (Categoria, u32, u8)>, String>{
            let ordenes = self.marketplace.get_ordenes()?;
            let ordenes = self.filtrar_validas(ordenes);
            let productos = self.marketplace.get_productos();
            let mut vector_categorias: Vec<(Categoria, u32, u8)> = Vec::new();
            let mut vector_puntuacion_total: Vec<(Categoria, u32)> = Vec::new();

            let _ = self.total_de_ventas_categorias(ordenes.clone(), productos.clone(), &mut vector_categorias, &mut vector_puntuacion_total)?;
            let _ = self.calificacion_promedio_categorias(&mut vector_categorias, vector_puntuacion_total)?;

            return Ok(vector_categorias)
        }

        /// Funcion que calcula el promedio de calificacion de cada una de las categorias.
        /// Divide la calificacion total por la cantidad total de ventas de la categoria.
        /// Errores posibles: Error al dividir
        fn calificacion_promedio_categorias(&self, vector_categorias: &mut Vec<(Categoria, u32, u8)>, vector_puntuacion_total: Vec<(Categoria, u32)>)-> Result<(), String>{
            for i in 0..vector_categorias.len(){
                let cantidad = vector_categorias[i].1;
                let total = vector_puntuacion_total[i].1;
                let promedio = total.checked_div(cantidad).ok_or("Error al dividir.")?;
                vector_categorias[i].2 = promedio as u8;
            }
            Ok(())
        }

        /// Funcion que procesa cada categoria de todas las ordenes de compra.
        fn total_de_ventas_categorias(&self, ordenes: Vec<OrdenCompra>, productos: Vec<Producto>, vector_categorias: &mut Vec<(Categoria, u32, u8)>, vector_puntuacion_total: &mut Vec<(Categoria, u32)>)->Result<(), String>{
            for orden in ordenes{
                let _ = self.procesar_categorias(&productos, vector_categorias, orden, vector_puntuacion_total)?;
            }
            return Ok(());
        }

        /// Funcion que se encarga de contar la cantidad de ventas de cada categoria y sus calificaciones totales,
        /// modificando el vector vector_categorias recibido por referencia mutable.
        /// El formato de la tupla es (categoria, total de ventas, total de calificacion).
        fn procesar_categorias(&self, productos: &Vec<Producto>, vector_categorias: &mut Vec<(Categoria, u32, u8)>, orden: OrdenCompra, vector_puntuacion_total: &mut Vec<(Categoria, u32)>)-> Result<(), String> {
            for (id, _) in orden.info_publicacion.1{
                if let Some(pos) = productos.iter().position(|producto| producto.id == id){
                    let categoria = productos.get(pos).unwrap().categoria.clone();
                    let _ = self.contar_categoria(categoria.clone(), vector_categorias, vector_puntuacion_total)?;
                    let _ = self.contar_puntuacion(categoria, vector_puntuacion_total, orden.puntuacion_del_comprador)?;
                }
            }
            return Ok(())
        }

        /// Funcion que suma la calificacion de una venta a la calificacion total de su categoria
        ///
        /// Errores posibles: Error al sumar
        fn contar_puntuacion(&self, categoria: Categoria, vector_puntuacion_total: &mut Vec<(Categoria, u32)>, puntuacion: Option<u8>) -> Result<(), String>{
            if let Some(nota) = puntuacion{
                let pos = vector_puntuacion_total.iter().position(|(categoria_del_vector,_)| *categoria_del_vector == categoria).unwrap();
                let mut nodo_vector = vector_puntuacion_total.get_mut(pos).unwrap();
                nodo_vector.1 = nodo_vector.1.checked_add(nota as u32).ok_or("Error al sumar.")?;
                Ok(())
            } else{
                Ok(())
            }
        }

        /// Funcion que incrementa en uno las ventas totales de una categoria
        ///
        /// Errores posibles: Error al incrementar
        fn contar_categoria(&self, categoria: Categoria, vector_categorias: &mut Vec<(Categoria, u32, u8)>, vector_puntuacion_total: &mut Vec<(Categoria, u32)>)-> Result<(), String>{
            if let Some(pos) = vector_categorias.iter().position(|(categoria_del_vector,_,_)| *categoria_del_vector == categoria){
                let mut nodo_vector = vector_categorias.get_mut(pos).unwrap();
                nodo_vector.1 = nodo_vector.1.checked_add(1).ok_or("Error al sumar.")?;
            }
            else {
                vector_categorias.push((categoria.clone(), 1, 0));
                vector_puntuacion_total.push((categoria, 0))
            }
            return Ok(());
        }

        /// Funcion que cuenta la cantidad de ordenes de compra realizada por cada uno de los compradores
        fn contar_cantidades(&self, usuarios: Vec<Usuario>) -> Vec<(AccountId, u32)>{
            let mut cantidades = Vec::new();

            for usuario in usuarios {
                cantidades.push((usuario.id_usuario, usuario.datos_comprador.unwrap().ordenes_de_compra.len() as u32))
            }

            cantidades
        }

        /// Funcion que filtra y descarta los usario que no poseen datos de comprador cargados
        fn filtrar_con_datos_comprador(&self, usuarios: Vec<Usuario>) -> Vec<Usuario>{
            usuarios.into_iter().filter(|usuario| usuario.datos_comprador.is_some()).collect()
        }
        
        /// Funcion que devuelve el id de los usuarios con mejor reputacion promedio del vector pasado por parametro
        fn calcular_5_mejores(&self, vec_contador: Vec<(AccountId, u8)>) -> Result<Vec<AccountId>, String>{
            let mut v = vec_contador;
            v.sort_by(|a, b| b.1.cmp(&a.1));
            let top: Vec<AccountId> = v.into_iter().take(5).map(|(acct, _score)| acct).collect();
            Ok(top)
        }

        /// Funcion que retorna la calificacion promedio de cada vendedor pasado por parametro, en fomarto (id_usuario, promedio)
        fn contar_promedios_vendedor(&self, vendedores: Vec<Usuario>) -> Result<Vec<(AccountId, u8)>, String>{
            let mut vector_contador = Vec::new();

            vendedores
            .iter()
            .for_each(|vendedor|{
                let id = vendedor.id_usuario;
                let promedio = self.promedio_reputacion(vendedor.datos_vendedor.clone().unwrap().reputacion_como_vendedor);
                let dato: (AccountId, u8) = (id, promedio);
                vector_contador.push(dato);
            });

            return Ok(vector_contador);
        }

        /// Funcion que retorna la calificacion promedio de cada comprador pasado por parametro, en fomarto (id_usuario, promedio)
        fn contar_promedios_comprador(&self, comprador: Vec<Usuario>) -> Result<Vec<(AccountId, u8)>, String>{
            let mut vector_contador = Vec::new();

            comprador
            .iter()
            .for_each(|comprador|{
                let id = comprador.id_usuario;
                let promedio = self.promedio_reputacion(comprador.datos_comprador.clone().unwrap().reputacion_como_comprador);
                let dato: (AccountId, u8) = (id, promedio);
                vector_contador.push(dato);
            });

            return Ok(vector_contador);
        }
        
        /// Funcion que filtra un listado de usuario, dejando solo aquellos que tengan el rol "Comp" o "Ambos" 
        fn filtrar_compradores(&self) -> Result<Vec<Usuario>,String> {
            let usuarios = self.marketplace.get_usuarios()?;
            let compradores: Vec<Usuario> = usuarios
                .into_iter()
                .filter(|usuario|{
                    (usuario.rol == Rol::Comp) || (usuario.rol == Rol::Ambos)
                })
                .collect();
            return Ok(compradores) 
        }

        /// Funcion que filtra un listado de usuario, dejando solo aquellos que tengan el rol "Vend" o "Ambos"
        fn filtrar_vendedores(&self) -> Result<Vec<Usuario>,String> {
            let usuarios = self.marketplace.get_usuarios()?;
            let vendedores: Vec<Usuario> = usuarios
                .into_iter()
                .filter(|usuario|{
                    (usuario.rol == Rol::Vend) || (usuario.rol == Rol::Ambos)
                })
                .collect();
            return Ok(vendedores) 
        }

        /// Funcion que retorna la nota promedio; como vendedor o como comprador, de un usuario
        fn promedio_reputacion(&self, puntajes: Vec<u8>) -> u8 {
            let n = puntajes.len();
            if n == 0 {
                return 0;
            }
            let suma: u32 = puntajes.iter().fold(0u32, |acc, &p| acc.saturating_add(p as u32));
            let mitad = (n as u32) / 2;
            let avg_u32 = (suma.saturating_add(mitad)).checked_div(n as u32).unwrap_or(0);
            avg_u32.min(u32::from(u8::MAX)) as u8
        }

        /// Funcion que procesa los productos de una orden de compra y cuenta sus cantidades
        ///
        /// Errores posibles: Overflow en la suma de las cantidades
        fn procesar_orden(&self, vector_contador: &mut Vec<(u32, u32)>, orden: OrdenCompra) -> Result<(), String> {
            for (id_producto, cantidad_producto) in orden.info_publicacion.1{
                if let Some(pos) = vector_contador.iter().position(|(id, _)| *id == id_producto){
                    let mut dato= *vector_contador.get_mut(pos).unwrap();
                    dato.1 = dato.1.checked_add(cantidad_producto).ok_or("Error al sumar.")?;
                    vector_contador.insert(pos, dato); 
                }
                else {
                    vector_contador.push((id_producto, cantidad_producto))
                }
            }
            return Ok(())
        }

        /// Funcion que filtra y descarta las ordenes de compra que esten en estado "Pendiente" o "Cancelada"
        fn filtrar_validas(&self, ordenes: Vec<OrdenCompra>) -> Vec<OrdenCompra> {
            ordenes.into_iter()
            .filter(|orden| orden.estado != EstadoCompra::Pendiente && orden.estado != EstadoCompra::Cancelada)
            .collect()
        }
    }

    //-------------Testing-------------//

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::prelude::string::String;
        use primer_contrato::{Categoria, OrdenCompra, Producto, EstadoCompra,Comprador,Vendedor};
        use ink::prelude::vec::Vec;
        use ink::env::account_id;
        use crate::segundo_contrato::AccountId;

        fn account(n: u8) -> AccountId {
            AccountId::from([n; 32])
        }
    
        #[ink::test]
        fn promedio_reputacion_empty_is_zero() {
            let contrato = SegundoContrato::new(account(0));
            let v: Vec<u8> = Vec::new();
            assert_eq!(contrato.promedio_reputacion(v), 0);
        }

        #[ink::test]
        fn promedio_reputacion_rounding_and_bounds() {
            let contrato = SegundoContrato::new(account(0));
            let v = vec![5u8, 4u8];
            assert_eq!(contrato.promedio_reputacion(v), 5u8);

            let v2 = vec![255u8, 255u8];
            assert_eq!(contrato.promedio_reputacion(v2), 255u8);
        }

        #[ink::test]
        fn filtrar_validas_filters_pending_and_cancelled() {
            let contrato = SegundoContrato::new(account(0));
            let o1 = OrdenCompra { id: 1, estado: EstadoCompra::Pendiente, cancelacion: (false, false), info_publicacion: (0, Vec::new(), 0, account(1)), id_comprador: account(2), calificaciones: (false, false), puntuacion_del_comprador: None };
            let o2 = OrdenCompra { id: 2, estado: EstadoCompra::Enviado, cancelacion: (false, false), info_publicacion: (0, Vec::new(), 0, account(1)), id_comprador: account(2), calificaciones: (false, false), puntuacion_del_comprador: None };
            let o3 = OrdenCompra { id: 3, estado: EstadoCompra::Cancelada, cancelacion: (false, false), info_publicacion: (0, Vec::new(), 0, account(1)), id_comprador: account(2), calificaciones: (false, false), puntuacion_del_comprador: None };

            let in_vec = vec![o1.clone(), o2.clone(), o3.clone()];
            let out = contrato.filtrar_validas(in_vec);
            assert_eq!(out.len(), 1);
            assert_eq!(out[0].id, 2);
        }

        #[ink::test]
        fn procesar_orden_pushes_new_products() {
            let contrato = SegundoContrato::new(account(0));
            let mut counter: Vec<(u32, u32)> = Vec::new();
            let pub_info = (1u32, vec![(10u32, 2u32), (20u32, 3u32)], 0u32, account(1));
            let orden = OrdenCompra { id: 1, estado: EstadoCompra::Enviado, cancelacion: (false, false), info_publicacion: pub_info, id_comprador: account(2), calificaciones: (false, false), puntuacion_del_comprador: None };
            contrato.procesar_orden(&mut counter, orden).expect("procesar_orden falla");
            assert_eq!(counter.len(), 2);
            assert!(counter.iter().any(|(id, qty)| *id == 10 && *qty == 2));
            assert!(counter.iter().any(|(id, qty)| *id == 20 && *qty == 3));
        }

        #[ink::test]
        fn contar_cantidades_counts_orders_per_user() {
            let contrato = SegundoContrato::new(account(0));
            let u1 = Usuario { id_usuario: account(1), nombre: String::from("a"), apellido: String::from("b"), direccion: String::from("c"), email: String::from("e"), rol: Rol::Comp, datos_comprador: Some(Comprador { ordenes_de_compra: vec![1,2,3], reputacion_como_comprador: vec![] }), datos_vendedor: None };
            let u2 = Usuario { id_usuario: account(2), nombre: String::from("x"), apellido: String::from("y"), direccion: String::from("z"), email: String::from("e2"), rol: Rol::Comp, datos_comprador: Some(Comprador { ordenes_de_compra: vec![10], reputacion_como_comprador: vec![] }), datos_vendedor: None };
            let res = contrato.contar_cantidades(vec![u1, u2]);
            assert_eq!(res.len(), 2);
            assert!(res.iter().any(|(id, qty)| *id == account(1) && *qty == 3));
            assert!(res.iter().any(|(id, qty)| *id == account(2) && *qty == 1));
        }

        #[ink::test]
        fn calcular_5_mejores_returns_top5() {
            let contrato = SegundoContrato::new(account(0));
            let mut v: Vec<(AccountId, u8)> = Vec::new();
            for i in 1..8u8 {
                v.push((account(i), i));
            }
            let top = contrato.calcular_5_mejores(v).expect("calculo falla");
            assert_eq!(top.len(), 5);
            assert_eq!(top[0], account(7));
            assert_eq!(top[4], account(3));
        }

        #[ink::test]
        fn contar_categoria_and_puntuacion_and_promedio() {
            let contrato = SegundoContrato::new(account(0));

            let p1 = Producto { id: 1, nombre: String::from("p1"), descripcion: String::from("d"), precio: 10, categoria: Categoria::Alimentos };
            let p2 = Producto { id: 2, nombre: String::from("p2"), descripcion: String::from("d2"), precio: 20, categoria: Categoria::Electrodomesticos };
            let productos = vec![p1.clone(), p2.clone()];

            let pub_info = (0u32, vec![(1u32, 2u32)], 0u32, account(3));
            let orden1 = OrdenCompra { id: 1, estado: EstadoCompra::Recibido, cancelacion: (false,false), info_publicacion: pub_info, id_comprador: account(4), calificaciones: (false,false), puntuacion_del_comprador: Some(4) };

            let mut vector_categorias: Vec<(Categoria, u32, u8)> = Vec::new();
            let mut vector_puntuacion_total: Vec<(Categoria, u32)> = Vec::new();

            contrato.total_de_ventas_categorias(vec![orden1.clone()], productos.clone(), &mut vector_categorias, &mut vector_puntuacion_total).expect("total falla");
            assert_eq!(vector_categorias.len(), 1);
            assert_eq!(vector_puntuacion_total.len(), 1);
            
            contrato.calificacion_promedio_categorias(&mut vector_categorias, vector_puntuacion_total).expect("promedio falla");
            assert_eq!(vector_categorias[0].2, 4u8);
        }

        #[ink::test]
        fn contar_promedios_vendedor_and_comprador() {
            let contrato = SegundoContrato::new(account(0));

            let vdata = Vendedor { productos: vec![], publicaciones: vec![], reputacion_como_vendedor: vec![5,4,3] };
            let vendedor = Usuario { id_usuario: account(10), nombre: String::from("v"), apellido: String::from("v"), direccion: String::from("d"), email: String::from("e"), rol: Rol::Vend, datos_comprador: None, datos_vendedor: Some(vdata) };
            let res_v = contrato.contar_promedios_vendedor(vec![vendedor]).expect("vendedor falla");
            assert_eq!(res_v.len(), 1);
            assert_eq!(res_v[0].0, account(10));

            let cdata = Comprador { ordenes_de_compra: vec![1], reputacion_como_comprador: vec![2,4] };
            let comprador = Usuario { id_usuario: account(11), nombre: String::from("c"), apellido: String::from("c"), direccion: String::from("d"), email: String::from("e"), rol: Rol::Comp, datos_comprador: Some(cdata), datos_vendedor: None };
            let res_c = contrato.contar_promedios_comprador(vec![comprador]).expect("comprador falla");
            assert_eq!(res_c.len(), 1);
            assert_eq!(res_c[0].0, account(11));
        }
    }
}