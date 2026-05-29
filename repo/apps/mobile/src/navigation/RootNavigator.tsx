import React from 'react';
import { View } from 'react-native';
import { NavigationContainer, RouteProp, useNavigation } from '@react-navigation/native';
import { createNativeStackNavigator, NativeStackNavigationProp } from '@react-navigation/native-stack';
import { createBottomTabNavigator } from '@react-navigation/bottom-tabs';
import { colors, typography } from '../theme/tokens';
import { SplashScreen } from '../screens/SplashScreen';
import { LoginScreen } from '../screens/LoginScreen';
import { RegisterScreen } from '../screens/RegisterScreen';
import { HomeScreen } from '../screens/HomeScreen';
import { ChatScreen } from '../screens/ChatScreen';
import { FindScreen } from '../screens/FindScreen';
import { RecommendationsScreen } from '../screens/RecommendationsScreen';
import { MessagesScreen } from '../screens/MessagesScreen';
import { MeScreen } from '../screens/MeScreen';
import { ProfileConfirmationScreen } from '../screens/ProfileConfirmationScreen';
import { SettingsScreen } from '../screens/SettingsScreen';
import { ComplianceScreen } from '../screens/ComplianceScreen';

const Stack = createNativeStackNavigator<RootStackParamList>();
const Tab = createBottomTabNavigator();

function MeScreenWrapper({ onLogout }: { onLogout: () => void }) {
  const navigation = useNavigation<NativeStackNavigationProp<RootStackParamList>>();
  return (
    <MeScreen
      onLogout={onLogout}
      onNavigateSettings={() => navigation.navigate('Settings')}
      onNavigateProfile={() => navigation.navigate('ProfileConfirmation')}
    />
  );
}

function SettingsScreenWrapper() {
  const navigation = useNavigation<NativeStackNavigationProp<RootStackParamList>>();
  return (
    <SettingsScreen
      onNavigateProfile={() => navigation.navigate('ProfileConfirmation')}
      onNavigateCompliance={() => navigation.navigate('Compliance')}
    />
  );
}

function MainTabs({ onLogout }: { onLogout: () => void }) {
  return (
    <Tab.Navigator
      screenOptions={{
        tabBarActiveTintColor: colors.primary,
        tabBarInactiveTintColor: colors.textSecondary,
        tabBarLabelStyle: {
          fontSize: typography.fontSize.xs,
          fontWeight: typography.fontWeight.medium,
        },
        headerStyle: {
          backgroundColor: colors.background,
        },
        headerTintColor: colors.textPrimary,
      }}
    >
      <Tab.Screen
        name="Home"
        component={HomeScreen}
        options={{ tabBarLabel: '首页', title: '首页 / Lumi' }}
      />
      <Tab.Screen
        name="Find"
        component={FindScreen}
        options={{ tabBarLabel: '找人', title: '找人' }}
      />
      <Tab.Screen
        name="Recommendations"
        component={RecommendationsScreen}
        options={{ tabBarLabel: '推荐', title: '推荐' }}
      />
      <Tab.Screen
        name="Messages"
        component={MessagesScreen}
        options={{ tabBarLabel: '消息', title: '消息' }}
      />
      <Tab.Screen
        name="Me"
        options={{ tabBarLabel: '我的', title: '我的' }}
      >
        {() => <MeScreenWrapper onLogout={onLogout} />}
      </Tab.Screen>
    </Tab.Navigator>
  );
}

export type RootStackParamList = {
  Splash: undefined;
  Auth: undefined;
  Main: undefined;
  Chat: { conversationId: string };
  ProfileConfirmation: undefined;
  Settings: undefined;
  Compliance: undefined;
};

export function RootNavigator() {
  const [isReady, setIsReady] = React.useState(false);
  const [hasSession, setHasSession] = React.useState(false);

  const handleBootComplete = (session: boolean) => {
    setHasSession(session);
    setIsReady(true);
  };

  const handleLoginSuccess = () => {
    setHasSession(true);
  };

  const handleLogout = () => {
    setHasSession(false);
  };

  if (!isReady) {
    return <SplashScreen onBootComplete={handleBootComplete} />;
  }

  return (
    <NavigationContainer>
      <Stack.Navigator screenOptions={{ headerShown: false }}>
        {hasSession ? (
          <>
            <Stack.Screen name="Main">
              {() => <MainTabs onLogout={handleLogout} />}
            </Stack.Screen>
            <Stack.Screen
              name="Chat"
              options={{
                headerShown: true,
                title: 'Lumi',
                headerStyle: { backgroundColor: colors.background },
                headerTintColor: colors.textPrimary,
              }}
            >
              {(props: { route: RouteProp<RootStackParamList, 'Chat'> }) => (
                <ChatScreen conversationId={props.route.params?.conversationId ?? ''} />
              )}
            </Stack.Screen>
            <Stack.Screen
              name="ProfileConfirmation"
              options={{
                headerShown: true,
                title: '画像确认',
                headerStyle: { backgroundColor: colors.background },
                headerTintColor: colors.textPrimary,
              }}
            >
              {() => <ProfileConfirmationScreen />}
            </Stack.Screen>
            <Stack.Screen
              name="Settings"
              options={{
                headerShown: true,
                title: '设置',
                headerStyle: { backgroundColor: colors.background },
                headerTintColor: colors.textPrimary,
              }}
            >
              {() => <SettingsScreenWrapper />}
            </Stack.Screen>
            <Stack.Screen
              name="Compliance"
              options={{
                headerShown: true,
                title: '数据合规',
                headerStyle: { backgroundColor: colors.background },
                headerTintColor: colors.textPrimary,
              }}
            >
              {() => <ComplianceScreen />}
            </Stack.Screen>
          </>
        ) : (
          <Stack.Group>
            <Stack.Screen name="Auth">
              {() => (
                <AuthStack onLoginSuccess={handleLoginSuccess} onRegisterSuccess={handleLoginSuccess} />
              )}
            </Stack.Screen>
          </Stack.Group>
        )}
      </Stack.Navigator>
    </NavigationContainer>
  );
}

function AuthStack({
  onLoginSuccess,
  onRegisterSuccess,
}: {
  onLoginSuccess: () => void;
  onRegisterSuccess: () => void;
}) {
  const [showRegister, setShowRegister] = React.useState(false);

  if (showRegister) {
    return <RegisterScreen onRegisterSuccess={onRegisterSuccess} onSwitchToLogin={() => setShowRegister(false)} />;
  }

  return (
    <View style={{ flex: 1 }}>
      <LoginScreen onLoginSuccess={onLoginSuccess} onSwitchToRegister={() => setShowRegister(true)} />
    </View>
  );
}
