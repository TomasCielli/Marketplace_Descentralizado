#![cfg_attr(not(feature = "std"), no_std, no_main)]

//IMPORTANTE: PERSISTENCIA DE DATOS
/* 
    1) Los datos que persistirán son solo aquellos bajo la etiqueta #[ink(storage)].
    2) Los datos simples solo persitirán si son variables del struct con la etiqueta #[ink(Storage)]
    3) Solo las estructuras StorageVec y Mapping persistirán su información. 
    4) Si se modifica un tipo de dato Vec o HashMap, se debe sobreescribir el respectivo Mapping o StorageVec.
        Ej: Modifico Usuario, debo reinsertarlo en el Mapping del Sistema. 


        CONSULTAS:
        
            1) Los productos deben estar previamente cargados en sistema? 
}

*/

#[ink::contract]
mod primer_contrato {
    use ink::storage::Mapping;
    use ink::storage::StorageVec;
    use ink::prelude::vec::Vec;
    use ink::prelude::string::String;


/////////////////////////// SISTEMA ///////////////////////////

    #[ink(storage)]
    pub struct PrimerContrato {
        usuarios: Mapping<AccountId, Usuario>, //Se persiste la información de los usuarios.     ----->  /Teo/ cambie u32 por acountId ya que son 2 tipos distintos para el programa 
        historial_publicaciones: StorageVec<Publicacion>, //Se persisten las publicaciones realizadas.
        historial_productos: Mapping<u32,Producto>, //Se persisten los productos de mis sistema. Team de que debería ser un Vec. 
        value: bool, //Si borras esto se rompe todo. En serio, fijate. (Hay que sacarle las fn, algún día se hará).  
    }
    impl PrimerContrato {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                usuarios: Mapping::default(),
                historial_publicaciones: StorageVec::new(),
                historial_productos: Mapping::default(),
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
        pub fn crear_publicacion(&mut self, productos_a_publicar: Vec<Producto>) -> Result<(), String> {
            let account_id = self.env().caller();
            if let Some(mut usuario) = self.usuarios.get(&account_id){
                let publicacion = usuario.crear_publicacion(productos_a_publicar)?;
                self.historial_publicaciones.push(&publicacion);
                self.usuarios.insert(account_id, &usuario);
                Ok(())
            }
            else {
                Err("No existe el usuario.".to_string())
            }

        }
        
        #[ink(message)]
        pub fn visualizar_productos(&self) -> Result<Vec<(Producto, u32)>, String>{
            let account_id = self.env().caller();
            if let Some(mut usuario) = self.usuarios.get(&account_id){
                let id_stock = usuario.visualizar_productos()?;
                let mut vector_productos_stock = Vec::new();
                for (id, stock) in id_stock{
                    let producto = self.historial_productos.get(id).unwrap().clone(); //Los productos deben estar previamente cargados en sistema
                    vector_productos_stock.push((producto, stock));
                }
                Ok(vector_productos_stock)
            }
            else {
                Err("No existe el usuario.".to_string())
            }

        }

        /// A message that can be called on instantiated contracts.
        /// This one flips the value of the stored `bool` from `true`
        /// to `false` and vice versa.
        #[ink(message)]
        pub fn flip(&mut self) {
            self.value = !self.value;
        }

            /// Simply returns the current value of our `bool`.
        #[ink(message)]
        pub fn get(&self) -> bool {
             self.value
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
                    Rol::Comprador | Rol::Ambos => Some(Comprador {         // si el rol es comprador o ambos se inicializan 
                        productos_comprados: Vec::new(),
                        calificaciones: Vec::new(),
                    }),
                    _ => None,
                },
                datos_vendedor: match rol {
                    Rol::Vendedor | Rol::Ambos => Some(Vendedor {            // si el rol es Vendedor o ambos se inicializan
                        stock_productos: Vec::new(),
                        productos_publicados: Vec::new(),
                        calificaciones: Vec::new(),
                    }),
                    _ => None,
                },
            }
        }

        fn crear_publicacion(&mut self, productos_a_publicar: Vec<Producto>) -> Result<Publicacion, String>{
            if self.rol == Rol::Comprador {  // <------------------------ TOBI DEJA DE LLORAR
                Err("El usuario no es vendedor.".to_string())
            }
            else {
                if let Some(ref mut datos_vendedor) = self.datos_vendedor{
                    datos_vendedor.crear_publicacion(productos_a_publicar)
                }
                else {
                    Err("No nos dejo poner otra cosa xd.".to_string())
                }
            }
        }

        fn visualizar_productos(&self) -> Result<Vec<(u32, u32)>, String>{
            if let Some(ref datos_vendedor) = self.datos_vendedor{
                Ok(datos_vendedor.stock_productos.clone())
            }
            else{
                Err("No es vendedor, no tiene productos.".to_string())
            }
        }
    }



/////////////////////////// COMPRADOR ///////////////////////////

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    struct Comprador{
        productos_comprados: Vec<u32>, 
        calificaciones: Vec<u8>, 
    }



/////////////////////////// VENDEDOR ///////////////////////////

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    struct Vendedor{
        stock_productos: Vec<(u32,u32)>, //Lo cambié a un Vec. Dolor de cabeza compilar. 
        productos_publicados: Vec<Publicacion>,
        calificaciones: Vec<u8>, 
    }
    impl Vendedor{
        fn crear_publicacion(&mut self, productos_a_publicar: Vec<Producto>) -> Result<Publicacion, String>{
            let precio_total = productos_a_publicar.iter().try_fold(0u64, |acc, producto| acc.checked_add(producto.precio)).ok_or("Overflow en suma de precios")?; 
            let publicacion = Publicacion{
                productos: productos_a_publicar,
                precio_total,
            };
            self.productos_publicados.push(publicacion.clone());
            Ok(publicacion)
        }

    }



/////////////////////////// ROL ///////////////////////////

    #[derive(Clone, PartialEq)]                // teo // agrege trait Clone 
    #[ink::scale_derive(Encode, Decode, TypeInfo)]                            
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    pub enum Rol {                                                              //warning a discutir   /* agregue pub al enum porque la impl de usuario la necesita */
        Ambos,
        Comprador, 
        Vendedor,
    }



/////////////////////////// PUBLICACION ///////////////////////////

    #[derive(Clone)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    struct Publicacion{
        productos: Vec<Producto>, 
        precio_total: u64, //Preguntar que tal se lleva la blockchain con numeros en punto flotante.  
    }



/////////////////////////// PRODUCTO ///////////////////////////

    #[derive(Clone)]                // teo // agrege trait Clone
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    pub struct Producto{
        id: u32,
        nombre: String,
        descripcion: String,
        stock: u32,
        precio: u64, //Podria ser flotante.
        categoria:String,
    }

    impl Producto {
        pub fn nuevo(id: u32, nombre: String, descripcion: String, stock: u32, precio: u64, categoria: String,) -> Producto {
            Producto {
                id,
                nombre,
                descripcion,
                stock,
                precio,
                categoria,
            }
        }
    }



/////////////////////////// ORDEN DE COMPRA ///////////////////////////

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    struct OrdenCompra{
        estado: EstadoCompra,
        cancelacion: (bool, bool),  //Para estado cancelada
        productos_comprados: Publicacion,
    }



/////////////////////////// ESTADO DE COMPRA ///////////////////////////

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