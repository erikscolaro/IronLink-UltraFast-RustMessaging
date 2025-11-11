import { useEffect } from 'react';
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
import { MessageDTO } from '../models/types';

export function useNotifications() {
  useEffect(() => {
    // Richiedi permessi per le notifiche all'avvio
    const setupNotifications = async () => {
      console.log('🔔 Inizializzazione notifiche...');
      
      try {
        let permissionGranted = await isPermissionGranted();
        console.log('🔔 Permessi già concessi?', permissionGranted);
        
        if (!permissionGranted) {
          console.log('🔔 Richiesta permessi notifiche...');
          const permission = await requestPermission();
          permissionGranted = permission === 'granted';
          console.log('🔔 Permessi concessi:', permissionGranted);
        }
        
        if (permissionGranted) {
          console.log('✅ Notifiche attive!');
        } else {
          console.log('❌ Notifiche negate dall\'utente');
        }
      } catch (error) {
        console.error('❌ Errore setup notifiche:', error);
      }
    };
    
    setupNotifications();
  }, []);

  const notifyNewMessage = async (message: MessageDTO) => {
    console.log('📬 Tentativo notifica nuovo messaggio');
    
    const permissionGranted = await isPermissionGranted();
    console.log('📬 Permessi:', permissionGranted);
    
    if (!permissionGranted) {
      console.log('📬 Notifica bloccata: permessi non concessi');
      return;
    }

    // Verifica che la finestra non sia in focus
    const isFocused = document.hasFocus();
    console.log('📬 Finestra in focus?', isFocused);
    
    if (isFocused) {
      console.log('📬 Notifica bloccata: finestra in focus');
      return; // Non notificare se l'app è in focus
    }

    // Prepara il contenuto della notifica
    const title = 'Nuovo messaggio';
    
    const body = message.content 
      ? message.content.substring(0, 100) + (message.content.length > 100 ? '...' : '')
      : 'Hai ricevuto un nuovo messaggio';

    console.log('📬 Invio notifica:', { title, body });

    try {
      await sendNotification({
        title,
        body,
      });
      console.log('✅ Notifica inviata!');
    } catch (error) {
      console.error('❌ Errore invio notifica:', error);
    }
  };

  const notifyNewInvitation = async (chatTitle?: string) => {
    console.log('📨 Tentativo notifica nuovo invito');
    
    const permissionGranted = await isPermissionGranted();
    console.log('📨 Permessi:', permissionGranted);
    
    if (!permissionGranted) {
      console.log('📨 Notifica bloccata: permessi non concessi');
      return;
    }

    const isFocused = document.hasFocus();
    console.log('📨 Finestra in focus?', isFocused);
    
    if (isFocused) {
      console.log('📨 Notifica bloccata: finestra in focus');
      return;
    }

    const title = 'Nuovo invito';
    const body = chatTitle 
      ? `Sei stato invitato a ${chatTitle}`
      : 'Hai ricevuto un nuovo invito a una chat';

    console.log('📨 Invio notifica:', { title, body });

    try {
      await sendNotification({
        title,
        body,
      });
      console.log('✅ Notifica invito inviata!');
    } catch (error) {
      console.error('❌ Errore invio notifica invito:', error);
    }
  };

  return {
    notifyNewMessage,
    notifyNewInvitation,
  };
}
