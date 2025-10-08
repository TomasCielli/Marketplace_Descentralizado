#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod segundo_contrato {

    use primer_contrato::{PrimerContratoRef, Usuario, Rol, EstadoCompra, OrdenCompra, Categoria, Producto};
    use ink::storage::Mapping;
    use ink::storage::StorageVec;
    use ink::prelude::vec::Vec;
    use ink::prelude::string::{String, ToString};
    use ink::env::call::FromAccountId;


    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct SegundoContrato {
        /// Stores a single `bool` value on the storage.
        marketplace: PrimerContratoRef,
    }
    
    impl SegundoContrato {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        // In `basic_contract_ref/lib.rs`
        
        #[ink(constructor)]
        pub fn new(primer_contrato_addr: AccountId) -> Self {
            let marketplace = PrimerContratoRef::from_account_id(primer_contrato_addr);
            Self { marketplace }
        }


        #[ink(message)]
        pub fn vendedores_mejor_reputacion(&self) -> Result <Vec <AccountId>, String>{
            let vendedores = self.filtrar_vendedores()?;
            let vec_contador: Vec<(AccountId, u8)> = self.contar_promedios_vendedor(vendedores)?;
            
            let top5: Vec<AccountId> = self.calcular_5_mejores(vec_contador)?;

            return Ok(top5);
        }

        #[ink(message)]
        pub fn productos_mas_vendidos(&self, top: Option<u32>) -> Result<Vec<(u32, u32)>, String>{
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
        pub fn compradores_mejor_reputacion(&self) -> Result<Vec<AccountId>, String>{
            let compradores = self.filtrar_compradores()?;
            let vec_contador: Vec<(AccountId, u8)> = self.contar_promedios_comprador(compradores)?;
            
            let top5: Vec<AccountId> = self.calcular_5_mejores(vec_contador)?;

            return Ok(top5);
        }

        #[ink(message)]
        pub fn cantidad_ordenes_por_usuarios(&self) -> Result<Vec<(AccountId, u32)>, String>{
            let usuarios = self.marketplace.get_usuarios()?;
            let usuarios = self.filtrar_con_datos_comprador(usuarios);

            if usuarios.is_empty(){
                return Err("No hay usuarios con datos de comprador cargados en sistema.".to_string());
            }

            let cantidades = self.contar_cantidades(usuarios);

            return Ok(cantidades);
        }

        #[ink(message)]
        pub fn estadisticas_por_categoria(&self) -> Result< Vec< (Categoria, u32, u8)>, String>{
            let ordenes = self.marketplace.get_ordenes()?;
            let ordenes = self.filtrar_validas(ordenes);
            let productos = self.marketplace.get_productos();
            let mut vector_categorias: Vec<(Categoria, u32, u8)> = Vec::new();
            let mut vector_puntuacion_total: Vec<(Categoria, u32)> = Vec::new();

            let _ = self.total_de_ventas_categorias(ordenes.clone(), productos.clone(), &mut vector_categorias, &mut vector_puntuacion_total)?;
            let _ = self.calificacion_promedio_categorias(&mut vector_categorias, vector_puntuacion_total)?;

            return Ok(vector_categorias)
        }

        fn calificacion_promedio_categorias(&self, vector_categorias: &mut Vec<(Categoria, u32, u8)>, vector_puntuacion_total: Vec<(Categoria, u32)>)-> Result<(), String>{
            for i in 0..vector_categorias.len(){
                let cantidad = vector_categorias[i].1;
                let total = vector_puntuacion_total[i].1;
                let promedio = total.checked_div(cantidad).ok_or("Error al dividir.")?;
                vector_categorias[i].2 = promedio as u8;
            }
            Ok(())
        }

        fn total_de_ventas_categorias(&self, ordenes: Vec<OrdenCompra>, productos: Vec<Producto>, vector_categorias: &mut Vec<(Categoria, u32, u8)>, vector_puntuacion_total: &mut Vec<(Categoria, u32)>)->Result<(), String>{
            for orden in ordenes{
                let _ = self.procesar_categorias(&productos, vector_categorias, orden, vector_puntuacion_total)?;
            }
            return Ok(());
        }

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


        fn contar_cantidades(&self, usuarios: Vec<Usuario>) -> Vec<(AccountId, u32)>{
            let mut cantidades = Vec::new();

            for usuario in usuarios {
                cantidades.push((usuario.id_usuario, usuario.datos_comprador.unwrap().ordenes_de_compra.len() as u32))
            }

            cantidades
        }

        fn filtrar_con_datos_comprador(&self, usuarios: Vec<Usuario>) -> Vec<Usuario>{
            usuarios.into_iter().filter(|usuario| usuario.datos_comprador.is_some()).collect()
        }
        
        fn calcular_5_mejores(&self, vec_contador: Vec<(AccountId, u8)>) -> Result<Vec<AccountId>, String>{
            let mut v = vec_contador;
            v.sort_by(|a, b| b.1.cmp(&a.1));
            let top: Vec<AccountId> = v.into_iter().take(5).map(|(acct, _score)| acct).collect();
            Ok(top)
        }

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

        fn procesar_orden(&self, vector_contador: &mut Vec<(u32, u32)>, orden: OrdenCompra) -> Result<(), String> {
            for (id_producto, cantidad_producto) in orden.info_publicacion.1{
                if let Some(pos) = vector_contador.iter().position(|(id, _)| *id == id_producto){
                    let mut dato= *vector_contador.get_mut(pos).unwrap();
                    dato.1 = dato.1.checked_add(cantidad_producto).ok_or("Error al sumar.")?;
                    vector_contador.insert(pos, dato); // <------ Capaz q lo borramos
                }
                else {
                    vector_contador.push((id_producto, cantidad_producto))
                }
            }
            return Ok(())
        }

        fn filtrar_validas(&self, ordenes: Vec<OrdenCompra>) -> Vec<OrdenCompra> {
            ordenes.into_iter()
            .filter(|orden| orden.estado != EstadoCompra::Pendiente && orden.estado != EstadoCompra::Cancelada)
            .collect()
        }
        


    }
}
