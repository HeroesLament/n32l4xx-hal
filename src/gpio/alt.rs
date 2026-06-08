#![allow(trivial_bounds)]

pub mod altmap;
macro_rules! extipin {
    ($( $(#[$attr:meta])* $PX:ident,)*) => {
        fn make_interrupt_source(&mut self, _syscfg: &mut $crate::pac::Afio) {
            match self {
                $(
                    $(#[$attr])*
                    Self::$PX(p) => p.make_interrupt_source(_syscfg),
                )*
                _ => {},
            }

        }

        fn trigger_on_edge(&mut self, _exti: &mut $crate::pac::Exti, _level: $crate::gpio::Edge) {
            match self {
                $(
                    $(#[$attr])*
                    Self::$PX(p) => p.trigger_on_edge(_exti, _level),
                )*
                _ => {},
            }
        }

        fn enable_interrupt(&mut self, _exti: &mut $crate::pac::Exti) {
            match self {
                $(
                    $(#[$attr])*
                    Self::$PX(p) => p.enable_interrupt(_exti),
                )*
                _ => {},
            }
        }
        fn disable_interrupt(&mut self, _exti: &mut $crate::pac::Exti) {
            match self {
                $(
                    $(#[$attr])*
                    Self::$PX(p) => p.disable_interrupt(_exti),
                )*
                _ => {},
            }
        }
        fn clear_interrupt_pending_bit(&mut self) {
            match self {
                $(
                    $(#[$attr])*
                    Self::$PX(p) => p.clear_interrupt_pending_bit(),
                )*
                _ => {},
            }
        }
        fn check_interrupt(&self) -> bool {
            match self {
                $(
                    $(#[$attr])*
                    Self::$PX(p) => p.check_interrupt(),
                )*
                _ => false,
            }
        }
    };
}
use extipin;

macro_rules! pin {
    ( $($(#[$docs:meta])* <$name:ident, $Otype:ident> for $(no: $NoPin:ident,)? [$(
        $(#[$attr:meta])* $PX:ident$(< Speed::$Speed:ident>)?,
    )*],)*) => {
        $(
            #[derive(Debug)]
            $(#[$docs])*
            pub enum $name {
                $(
                    None($NoPin<$Otype>),
                )?

                $(
                    $(#[$attr])*
                    $PX(gpio::$PX<$crate::gpio::Alternate<$Otype>>),
                )*
            }

            impl crate::Sealed for $name { }

            #[allow(unreachable_patterns)]
            impl $crate::gpio::ReadPin for $name {
                fn is_low(&self) -> bool {
                    match self {
                        $(
                            $(#[$attr])*
                            Self::$PX(p) => p.is_low(),
                        )*
                        _ => false,
                    }
                }
            }

            #[allow(unreachable_patterns)]
            impl $crate::gpio::PinSpeed for $name {
                fn set_speed(&mut self, _speed: $crate::gpio::Speed) {
                    match self {
                        $(
                            $(#[$attr])*
                            Self::$PX(p) => p.set_speed(_speed),
                        )*
                        _ => {}
                    }
                }
            }

            #[allow(unreachable_patterns)]
            impl $crate::gpio::ExtiPin for $name {
                extipin! { $( $(#[$attr])* $PX, )* }
            }

            $(
                impl<T> From<$NoPin<T>> for $name {
                    fn from(p: $NoPin<T>) -> Self {
                        Self::None(p)
                    }
                }
            )?

            $(
                $(#[$attr])*
                impl<MODE> From<gpio::$PX<MODE>> for $name
                where
                    MODE: $crate::gpio::marker::NotAlt + $crate::gpio::PinMode
                {
                    fn from(p: gpio::$PX<MODE>) -> Self {
                        Self::$PX(p.into_mode() $(.speed($crate::gpio::Speed::$Speed))?)
                    }
                }

                $(#[$attr])*
                impl From<gpio::$PX<$crate::gpio::Alternate<$Otype>>> for $name {
                    fn from(p: gpio::$PX<$crate::gpio::Alternate<$Otype>>) -> Self {
                        Self::$PX(p $(.speed($crate::gpio::Speed::$Speed))?)
                    }
                }

                $(#[$attr])*
                #[allow(irrefutable_let_patterns)]
                impl<MODE> TryFrom<$name> for gpio::$PX<MODE>
                where
                    MODE: $crate::gpio::PinMode,
                    $crate::gpio::Alternate<$Otype>: $crate::gpio::PinMode,
                {
                    type Error = ();

                    fn try_from(a: $name) -> Result<Self, Self::Error> {
                        if let $name::$PX(p) = a {
                            Ok(p.into_mode())
                        } else {
                            Err(())
                        }
                    }
                }
            )*
        )*
    };

    ( $($(#[$docs:meta])* <$name:ident> default:$DefaultOtype:ident for $(no: $NoPin:ident,)? [$(
            $(#[$attr:meta])* $PX:ident,
    )*],)*) => {
        $(
            #[derive(Debug)]
            $(#[$docs])*
            pub enum $name<Otype = $DefaultOtype> {
                $(
                    None($NoPin<Otype>),
                )?

                $(
                    $(#[$attr])*
                    $PX(gpio::$PX<$crate::gpio::Alternate<Otype>>),
                )*
            }

            impl<Otype> crate::Sealed for $name<Otype> { }

            #[allow(unreachable_patterns)]
            impl<Otype> $crate::gpio::ReadPin for $name<Otype> {
                fn is_low(&self) -> bool {
                    match self {
                        $(
                            $(#[$attr])*
                            Self::$PX(p) => p.is_low(),
                        )*
                        _ => false,
                    }
                }
            }

            #[allow(unreachable_patterns)]
            impl<Otype> $crate::gpio::PinSpeed for $name<Otype> {
                fn set_speed(&mut self, _speed: $crate::gpio::Speed) {
                    match self {
                        $(
                            $(#[$attr])*
                            Self::$PX(p) => p.set_speed(_speed),
                        )*
                        _ => {}
                    }
                }
            }

            #[allow(unreachable_patterns)]
            impl<Otype> $crate::gpio::ExtiPin for $name<Otype> {
                extipin! { $( $(#[$attr])* $PX, )* }
            }

            $(
                impl<T,V> From<$NoPin<T>> for $name<V> {
                    fn from(_: $NoPin<T>) -> Self {
                        Self::None($NoPin::<V>(PhantomData{}))
                    }
                }
            )?

            $(
                $(#[$attr])*
                impl<MODE, Otype> From<gpio::$PX<MODE>> for $name<Otype>
                where
                    MODE: $crate::gpio::marker::NotAlt + $crate::gpio::PinMode,
                    $crate::gpio::Alternate< Otype>: $crate::gpio::PinMode,
                {
                    fn from(p: gpio::$PX<MODE>) -> Self {
                        Self::$PX(p.into_mode())
                    }
                }

                $(#[$attr])*
                impl<Otype> From<gpio::$PX<$crate::gpio::Alternate<Otype>>> for $name<Otype> {
                    fn from(p: gpio::$PX<$crate::gpio::Alternate<Otype>>) -> Self {
                        Self::$PX(p)
                    }
                }

                $(#[$attr])*
                #[allow(irrefutable_let_patterns)]
                impl<MODE, Otype> TryFrom<$name<Otype>> for gpio::$PX<MODE>
                where
                    MODE: $crate::gpio::PinMode,
                    $crate::gpio::Alternate<Otype>: $crate::gpio::PinMode,
                {
                    type Error = ();

                    fn try_from(a: $name<Otype>) -> Result<Self, Self::Error> {
                        if let $name::$PX(p) = a {
                            Ok(p.into_mode())
                        } else {
                            Err(())
                        }
                    }
                }
            )*
        )*
    };
}
use pin;

// CAN pins
pub trait CanCommon {
    type Rx;
    type Tx;
}

// Serial pins
pub trait SerialAsync {
    /// Receive
    type Rx<Itype>;
    /// Transmit
    type Tx<Otype>;

}
/// Synchronous mode
pub trait SerialSync {
    /// Receive
    type Rx<Itype>;
    /// Transmit
    type Tx<Otype>;    
    /// Clock
    type Ck;
}
/// Hardware flow control (RS232)
pub trait SerialRs232 {
    /// Receive
    type Rx<Itype>;
    /// Transmit
    type Tx<Otype>;    
    /// Clear To Send
    type Cts;
    /// Request To Send
    type Rts;
}

// I2C pins
pub trait I2cCommon {
    type Scl;
    type Sda;
    type Smba;
}

// I2S pins
pub trait I2sCommon {
    type Ck: crate::gpio::PinSpeed;
    type Sd;
    type Ws: crate::gpio::ReadPin + crate::gpio::ExtiPin;
}
pub trait I2sMaster {
    type Mck;
}
pub trait I2sExtPin {
    type ExtSd;
}

// QuadSPI pins

pub trait QuadSpi {
    type Io0: crate::gpio::PinSpeed;
    type Io1: crate::gpio::PinSpeed;
    type Io2: crate::gpio::PinSpeed;
    type Io3: crate::gpio::PinSpeed;
    type Ncs: crate::gpio::PinSpeed;
}


// SPI pins
pub trait SpiCommon {
    type Miso;
    type Mosi;
    type Nss;
    type Sck;
}

// Timer pins

/// Input capture / Output compare channel `C`
pub trait TimCPin<const C: u8> {
    type Ch<Otype>;
}

/// Complementary output channel `C`
pub trait TimNCPin<const C: u8> {
    type ChN<Otype>;
}

/// Break input
pub trait TimBkin {
    type Bkin;
}

/// External trigger timer input
pub trait TimEtr {
    type Etr;
}

