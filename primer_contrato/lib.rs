#![cfg_attr(not(feature = "std"), no_std, no_main)]

//IMPORTANTE: PERSISTENCIA DE DATOS
/* 
    1) Los datos que persistirán son solo aquellos bajo la etiqueta #[ink(storage)].
    2) Los datos simples solo persitirán si son variables del struct con la etiqueta #[ink(Storage)]
    3) Solo las estructuras StorageVec y Mapping persistirán su información. 
    4) Si se modifica un tipo de dato Vec o HashMap, se debe sobreescribir el respectivo Mapping o StorageVec.
        Ej: Modifico Usuario, debo reinsertarlo en el Mapping del Sistema. 
*/

#[ink::contract]
mod primer_contrato {

    use ink::storage::Mapping;
    use ink::prelude::collections::HashMap; //No deberiamos usar nunca un HashMap. No me compila, más fácil clavar un Vec. 
    use ink::storage::StorageVec;
    use ink::prelude::vec::Vec;
    use ink::prelude::string::String;
    
    #[ink(storage)]
    pub struct PrimerContrato {
        usuarios: Mapping<u32, Usuario>, //Se persiste la información de los usuarios.
        historial_publicacioes: StorageVec<Publicacion>, //Se persisten las publicaciones realizadas.
        historial_productos: Mapping<u32,Producto>, //Se persisten los productos de mis sistema. Team de que debería ser un Vec. 
        value: bool, //Si borras esto se rompe todo. En serio, fijate. (Hay que sacarle las fn, algún día se hará).  
    }

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

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    struct Comprador{
        productos_comprados: Vec<u32>, 
        calificaciones: Vec<u8>, 
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    struct Vendedor{
        stock_productos: Vec<(u32,u32)>, //Lo cambié a un Vec. Dolor de cabeza compilar. 
        productos_publicados: Vec<Publicacion>,
        calificaciones: Vec<u8>, 
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    enum Rol {
        Ambos,
        Comprador, 
        Vendedor,
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    struct Publicacion{
        productos: Vec<Producto>, 
        precio_total: u128, //Preguntar que tal se lleva la blockchain con numeros en punto flotante.  
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    struct Producto{
        nombre: String,
        descripcion: String,
        stock: u32,
        precio: u128, //Podria ser flotante. 
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    struct OrdenCompra{
        estado: EstadoCompra,
        cancelacion: (bool, bool),  //Para estado cancelada
        productos_comprados: Publicacion,
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std",derive(ink::storage::traits::StorageLayout))]
    enum EstadoCompra{
        Pendiente,
        Enviado,
        Recibido,
        Cancelada,
    }

    impl PrimerContrato {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                usuarios: Mapping::default(),
                historial_publicacioes: StorageVec::new(),
                historial_productos: Mapping::default(),
                value: false,
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