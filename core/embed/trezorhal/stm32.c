/*
 * This file is part of the Trezor project, https://trezor.io/
 *
 * Copyright (c) SatoshiLabs
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

#include STM32_HAL_H

#include "rng.h"

const uint8_t AHBPrescTable[16] = {0, 0, 0, 0, 0, 0, 0, 0,
                                   1, 2, 3, 4, 6, 7, 8, 9};
const uint8_t APBPrescTable[8] = {0, 0, 0, 0, 1, 2, 3, 4};

#ifdef STM32F427xx
#define CORE_CLOCK_MHZ 168U
#elif STM32F405xx
#define CORE_CLOCK_MHZ 120U
#elif STM32H757xx
#define CORE_CLOCK_MHZ 120U
#else
#error Unsupported MCU
#endif


#pragma GCC optimize( \
    "no-stack-protector")  // applies to all functions in this file

#if defined(STM32F427xx) || defined(STM32F405xx)
uint32_t SystemCoreClock = CORE_CLOCK_MHZ * 1000000U;
void SystemInit(void) {
  // set flash wait states for an increasing HCLK frequency -- reference RM0090
  // section 3.5.1
  FLASH->ACR = FLASH_ACR_LATENCY_5WS;
  // wait until the new wait state config takes effect -- per section 3.5.1
  // guidance
  while ((FLASH->ACR & FLASH_ACR_LATENCY) != FLASH_ACR_LATENCY_5WS)
    ;
  // configure main PLL; assumes HSE is 8 MHz; this should evaluate to
  // 0x27402a04 -- reference RM0090 section 7.3.2
  RCC->PLLCFGR =
      (RCC_PLLCFGR_RST_VALUE & ~RCC_PLLCFGR_PLLQ & ~RCC_PLLCFGR_PLLSRC &
       ~RCC_PLLCFGR_PLLP & ~RCC_PLLCFGR_PLLN & ~RCC_PLLCFGR_PLLM) |
      (7U << RCC_PLLCFGR_PLLQ_Pos)    // Q = 7
      | RCC_PLLCFGR_PLLSRC_HSE        // PLLSRC = HSE
      | (0U << RCC_PLLCFGR_PLLP_Pos)  // P = 2 (two bits, 00 means PLLP = 2)
      | (CORE_CLOCK_MHZ << RCC_PLLCFGR_PLLN_Pos)  // N = CORE_CLOCK_MHZ
      | (4U << RCC_PLLCFGR_PLLM_Pos);             // M = 4
  // enable spread spectrum clock for main PLL
  RCC->SSCGR = RCC_SSCGR_SSCGEN | (44 << RCC_SSCGR_INCSTEP_Pos) |
               (250 << RCC_SSCGR_MODPER_Pos);
  // enable clock security system, HSE clock, and main PLL
  RCC->CR |= RCC_CR_CSSON | RCC_CR_HSEON | RCC_CR_PLLON;
  // wait until PLL and HSE ready
  while ((RCC->CR & (RCC_CR_PLLRDY | RCC_CR_HSERDY)) !=
         (RCC_CR_PLLRDY | RCC_CR_HSERDY))
    ;
  // APB2=2, APB1=4, AHB=1, system clock = main PLL
  const uint32_t cfgr = RCC_CFGR_PPRE2_DIV2 | RCC_CFGR_PPRE1_DIV4 |
                        RCC_CFGR_HPRE_DIV1 | RCC_CFGR_SW_PLL;
  RCC->CFGR = cfgr;
  // wait until PLL is system clock and also verify that the pre-scalers were
  // set
  while (RCC->CFGR != (RCC_CFGR_SWS_PLL | cfgr))
    ;
  // turn off the HSI as it is now unused (it will be turned on again
  // automatically if a clock security failure occurs)
  RCC->CR &= ~RCC_CR_HSION;
  // wait until ths HSI is off
  while ((RCC->CR & RCC_CR_HSION) == RCC_CR_HSION)
    ;
  // init the TRNG peripheral
  rng_init();
  // set CP10 and CP11 to enable full access to the fpu coprocessor; ARMv7-M
  // Architecture Reference Manual section B3.2.20
  SCB->CPACR |= ((3U << 22) | (3U << 20));
}

// from util.s
extern void shutdown_privileged(void);

void PVD_IRQHandler(void) {
  TIM1->CCR1 = 0;  // turn off display backlight
  shutdown_privileged();
}

#elif defined(STM32H757xx)

void sys_clktree_cfg(void)
{
  RCC_ClkInitTypeDef RCC_ClkInitStruct;
  RCC_OscInitTypeDef RCC_OscInitStruct;
  RCC_PeriphCLKInitTypeDef PeriphClkInitStruct;
  HAL_StatusTypeDef ret = HAL_OK;
  
  /*!< Supply configuration update enable */
  // HAL_PWREx_ConfigSupply(PWR_DIRECT_SMPS_SUPPLY);
  HAL_PWREx_ConfigSupply(PWR_LDO_SUPPLY);

  /* The voltage scaling allows optimizing the power consumption when the device is 
     clocked below the maximum system frequency, to update the voltage scaling value 
     regarding system frequency refer to product datasheet.  */
  __HAL_PWR_VOLTAGESCALING_CONFIG(PWR_REGULATOR_VOLTAGE_SCALE1);

  while(!__HAL_PWR_GET_FLAG(PWR_FLAG_VOSRDY)) {
    ;
  }
  
  /* Enable HSE Oscillator and activate PLL with HSE as source */
  RCC_OscInitStruct.OscillatorType = RCC_OSCILLATORTYPE_HSI48|RCC_OSCILLATORTYPE_HSE;
  RCC_OscInitStruct.HSEState = RCC_HSE_ON;
  RCC_OscInitStruct.HSI48State = RCC_HSI48_ON;
  RCC_OscInitStruct.HSIState = RCC_HSI_OFF;
  RCC_OscInitStruct.CSIState = RCC_CSI_OFF;
  RCC_OscInitStruct.PLL.PLLState = RCC_PLL_ON;
  RCC_OscInitStruct.PLL.PLLSource = RCC_PLLSOURCE_HSE;

  RCC_OscInitStruct.PLL.PLLM = 5;
  RCC_OscInitStruct.PLL.PLLN = 160;
  RCC_OscInitStruct.PLL.PLLFRACN = 0;
  RCC_OscInitStruct.PLL.PLLP = 2;
  RCC_OscInitStruct.PLL.PLLR = 2;
  RCC_OscInitStruct.PLL.PLLQ = 4;

  RCC_OscInitStruct.PLL.PLLVCOSEL = RCC_PLL1VCOWIDE;
  RCC_OscInitStruct.PLL.PLLRGE = RCC_PLL1VCIRANGE_2;
  ret = HAL_RCC_OscConfig(&RCC_OscInitStruct);
  if(ret != HAL_OK){
    ;//error
  }
  
/* Select PLL as system clock source and configure  bus clocks dividers */
  RCC_ClkInitStruct.ClockType = (RCC_CLOCKTYPE_SYSCLK | RCC_CLOCKTYPE_HCLK | RCC_CLOCKTYPE_D1PCLK1 | RCC_CLOCKTYPE_PCLK1 | \
                                 RCC_CLOCKTYPE_PCLK2  | RCC_CLOCKTYPE_D3PCLK1);

  RCC_ClkInitStruct.SYSCLKSource = RCC_SYSCLKSOURCE_PLLCLK;
  RCC_ClkInitStruct.SYSCLKDivider = RCC_SYSCLK_DIV1;
  RCC_ClkInitStruct.AHBCLKDivider = RCC_HCLK_DIV2;
  RCC_ClkInitStruct.APB3CLKDivider = RCC_APB3_DIV2;  
  RCC_ClkInitStruct.APB1CLKDivider = RCC_APB1_DIV2; 
  RCC_ClkInitStruct.APB2CLKDivider = RCC_APB2_DIV2; 
  RCC_ClkInitStruct.APB4CLKDivider = RCC_APB4_DIV2; 
  ret = HAL_RCC_ClockConfig(&RCC_ClkInitStruct, FLASH_LATENCY_4);
  if(ret != HAL_OK){
    ;//error
  }

  PeriphClkInitStruct.PeriphClockSelection = RCC_PERIPHCLK_USB;
  PeriphClkInitStruct.UsbClockSelection = RCC_USBCLKSOURCE_HSI48;
  if (HAL_RCCEx_PeriphCLKConfig(&PeriphClkInitStruct) != HAL_OK){
    ;//error
  }

 /*
  Note : The activation of the I/O Compensation Cell is recommended with communication  interfaces
          (GPIO, SPI, FMC, QSPI ...)  when  operating at  high frequencies(please refer to product datasheet)       
          The I/O Compensation Cell activation  procedure requires :
        - The activation of the CSI clock
        - The activation of the SYSCFG clock
        - Enabling the I/O Compensation Cell : setting bit[0] of register SYSCFG_CCCSR
 */
 
  /*activate CSI clock mondatory for I/O Compensation Cell*/  
  __HAL_RCC_CSI_ENABLE() ;
    
  /* Enable SYSCFG clock mondatory for I/O Compensation Cell */
  __HAL_RCC_SYSCFG_CLK_ENABLE() ;
  
  /* Enables the I/O Compensation Cell */    
  HAL_EnableCompensationCell();  

  SystemCoreClockUpdate();
}

void SystemInitAtStartup(void) {
  // init sys clk tree
  sys_clktree_cfg();
  // init the TRNG peripheral
  rng_init();
  // set CP10 and CP11 to enable full access to the fpu coprocessor; ARMv7-M
  // Architecture Reference Manual section B3.2.20
  SCB->CPACR |= ((3U << 22) | (3U << 20));

}

// from util.s
extern void shutdown_privileged(void);
// H757 handler
void PVD_AVD_IRQHandler(void) {
  TIM1->CCR1 = 0;  // turn off display backlight
  HAL_PWR_PVD_IRQHandler();
  shutdown_privileged();
}
#endif